use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{HashMap, HashSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    ops::Range,
    sync::Arc,
    sync::mpsc,
    time::Duration,
};

#[cfg(test)]
std::thread_local! {
    static TEXTURE_IDENTITY_EQ_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

use super::{
    GpuContext, GpuContextGeneration, GpuImage, GpuInitializationResourceSnapshot, GpuLayer,
    GpuLayerError, GpuQuad, PixelRect,
    images::{GpuImagePipeline, image_layer},
    quads::{INSTANCE_SIZE, encode_instance},
    text::{
        GpuText, GpuTextAtlasMetrics, GpuTextCatalogStatus, GpuTextConfig, GpuTextCpuFontMetrics,
        GpuTextOwnerMaterialization, GpuTextPrepareReport,
    },
};
use rterm_fonts::{FontCatalog, FontConfig};
use rterm_render_cpu::{
    DamageRegion, DecodedImage, ImageDrawPlan, ImageTiePolicy, RenderGeometry,
    TerminalRenderSnapshot, TextPaintConfig, gpu_image_draw_plan, image_draw_pixel,
};

const INSTANCE_STRIDE: wgpu::BufferAddress = INSTANCE_SIZE as wgpu::BufferAddress;
const MIN_INSTANCE_CAPACITY: usize = 64;
const INSTANCE_WRITE_ALIGNMENT: usize = 4;
pub const DEFAULT_GPU_INSTANCE_BYTE_BUDGET: usize = 64 * 1024 * 1024;
pub const DEFAULT_GPU_IMAGE_BYTE_BUDGET: usize = 64 * 1024 * 1024;
pub const DEFAULT_GPU_READBACK_BYTE_BUDGET: usize = 64 * 1024 * 1024;
const MAX_GPU_READBACK_WAIT: Duration = Duration::from_secs(5);

#[cfg(any(test, debug_assertions))]
const INVALID_IMAGE_SHADER_FOR_TEST: &str = "this is intentionally invalid WGSL";

const QUAD_SHADER: &str = r"
struct Viewport {
    size: vec2<f32>,
};
@group(0) @binding(0)
var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let corners = array(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let pixel = input.rect.xy + corners[vertex_index] * input.rect.zw;
    let normalized = pixel / viewport.size;
    var output: VertexOutput;
    output.position = vec4<f32>(
        normalized.x * 2.0 - 1.0,
        1.0 - normalized.y * 2.0,
        0.0,
        1.0,
    );
    output.color = input.color;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
";

const IMAGE_SHADER: &str = r"
struct Viewport {
    size: vec2<f32>,
};
@group(0) @binding(0)
var<uniform> viewport: Viewport;
@group(1) @binding(0)
var image_texture: texture_2d<f32>;

struct VertexInput {
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vertex_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let corners = array(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertex_index];
    let pixel = input.rect.xy + corner * input.rect.zw;
    let normalized = pixel / viewport.size;
    var output: VertexOutput;
    output.position = vec4<f32>(
        normalized.x * 2.0 - 1.0,
        1.0 - normalized.y * 2.0,
        0.0,
        1.0,
    );
    output.local = corner;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(image_texture);
    let coordinate = min(
        vec2<i32>(input.local * vec2<f32>(dimensions)),
        vec2<i32>(dimensions) - vec2<i32>(1),
    );
    let color = textureLoad(image_texture, coordinate, 0);
    if color.a == 0.0 {
        discard;
    }
    return color;
}
";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum PrimitiveKind {
    Quad,
    Image,
    TextureImage(usize),
}

#[derive(Clone, Debug)]
pub(crate) struct TextureIdentity {
    digest: u64,
    width: u32,
    height: u32,
    pixels: Arc<[u8]>,
}

impl TextureIdentity {
    pub(crate) fn from_rgba(width: u32, height: u32, pixels: Arc<[u8]>) -> Self {
        let mut hasher = DefaultHasher::new();
        width.hash(&mut hasher);
        height.hash(&mut hasher);
        pixels.hash(&mut hasher);
        Self {
            digest: hasher.finish(),
            width,
            height,
            pixels,
        }
    }
}

impl PartialEq for TextureIdentity {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(test)]
        TEXTURE_IDENTITY_EQ_CALLS.with(|calls| calls.set(calls.get().saturating_add(1)));
        self.digest == other.digest
            && self.width == other.width
            && self.height == other.height
            && self.pixels == other.pixels
    }
}

impl Eq for TextureIdentity {}

impl Hash for TextureIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.digest.hash(state);
        self.width.hash(state);
        self.height.hash(state);
    }
}

#[derive(Clone, Debug)]
enum GraphNode {
    Quad {
        sequence: u64,
        quad: GpuQuad,
    },
    Image {
        sequence: u64,
        image: GpuImage,
    },
    TextureImage {
        sequence: u64,
        layer: GpuLayer,
        texture: Option<TextureIdentity>,
        plan: Arc<ImageDrawPlan>,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum GraphNodeKind {
    Primitive,
    Whole,
    Fragment,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct GraphNodeOrderKey {
    layer: u8,
    kind: GraphNodeKind,
    z_index: i32,
    id_group: u8,
    kitty_image_id: u32,
    parent_index: usize,
    fragment_index: usize,
    sequence: u64,
}

impl GraphNode {
    fn layer(&self) -> GpuLayer {
        match self {
            Self::Quad { quad, .. } => quad.layer(),
            Self::Image { image, .. } => image.layer(),
            Self::TextureImage { layer, .. } => *layer,
        }
    }

    const fn sequence(&self) -> u64 {
        match self {
            Self::Quad { sequence, .. }
            | Self::Image { sequence, .. }
            | Self::TextureImage { sequence, .. } => *sequence,
        }
    }

    fn rect(&self) -> PixelRect {
        match self {
            Self::Quad { quad, .. } => quad.rect(),
            Self::Image { image, .. } => image.rect(),
            Self::TextureImage { plan, .. } => PixelRect::new(
                plan.destination_x,
                plan.destination_y,
                plan.width,
                plan.height,
            ),
        }
    }
}

/// CPU-side display list for the native GPU terminal passes.
#[derive(Clone, Debug)]
pub struct RenderGraph {
    width: u32,
    height: u32,
    nodes: Vec<GraphNode>,
    next_sequence: u64,
}

impl RenderGraph {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            nodes: Vec::new(),
            next_sequence: 0,
        }
    }

    pub fn push_quad(&mut self, quad: GpuQuad) {
        let sequence = self.allocate_sequence();
        self.nodes.push(GraphNode::Quad { sequence, quad });
    }

    pub fn push_image(&mut self, image: GpuImage) {
        let sequence = self.allocate_sequence();
        self.nodes.push(GraphNode::Image { sequence, image });
    }

    /// Adds normalized image draws from the terminal snapshot. This deliberately
    /// calls the same attachment/fragment planner as the CPU reference backend.
    pub fn push_snapshot_images(
        &mut self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        animation_frame: usize,
        animation_elapsed_ms: Option<u64>,
    ) {
        for plan in gpu_image_draw_plan(snapshot, geometry, animation_frame, animation_elapsed_ms) {
            let sequence = self.allocate_sequence();
            self.nodes.push(GraphNode::TextureImage {
                sequence,
                layer: image_layer(plan.z_index),
                texture: None,
                plan: Arc::new(plan),
            });
        }
    }

    pub(crate) fn push_background_texture(
        &mut self,
        decoded: Arc<DecodedImage>,
        texture: TextureIdentity,
        destination: PixelRect,
    ) {
        let sequence = self.allocate_sequence();
        self.nodes.push(GraphNode::TextureImage {
            sequence,
            layer: GpuLayer::PaneBackground,
            texture: Some(texture),
            plan: Arc::new(ImageDrawPlan {
                destination_x: destination.x,
                destination_y: destination.y,
                width: destination.width,
                height: destination.height,
                sample_source_x: 0,
                sample_source_y: 0,
                sample_target_x: 0,
                sample_target_y: 0,
                sample_source_width: decoded.width,
                sample_source_height: decoded.height,
                sample_destination_width: destination.width,
                sample_destination_height: destination.height,
                z_index: 0,
                kitty_image_id: None,
                parent_index: 0,
                fragment_index: 0,
                tie_policy: ImageTiePolicy::Whole,
                stable_order: 0,
                decoded,
            }),
        });
    }

    #[must_use]
    pub fn planned_image_draw_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| matches!(node, GraphNode::TextureImage { .. }))
            .count()
    }

    #[must_use]
    pub fn planned_image_destinations(&self) -> Vec<PixelRect> {
        self.nodes
            .iter()
            .filter_map(|node| match node {
                GraphNode::TextureImage { plan, .. } => Some(PixelRect::new(
                    plan.destination_x,
                    plan.destination_y,
                    plan.width,
                    plan.height,
                )),
                GraphNode::Quad { .. } | GraphNode::Image { .. } => None,
            })
            .collect()
    }

    pub fn replace_quad(&mut self, quad_index: usize, replacement: GpuQuad) {
        if let Some(GraphNode::Quad { quad, .. }) = self
            .nodes
            .iter_mut()
            .filter(|node| matches!(node, GraphNode::Quad { .. }))
            .nth(quad_index)
        {
            *quad = replacement;
        }
    }

    #[must_use]
    pub const fn ordered_layers(&self) -> &'static [GpuLayer] {
        &GpuLayer::ORDERED
    }

    #[must_use]
    pub fn ordered_images(&self) -> Vec<GpuImage> {
        let mut images = self
            .nodes
            .iter()
            .filter(|node| matches!(node, GraphNode::Image { .. }))
            .collect::<Vec<_>>();
        images.sort_by(|left, right| node_order(left, right));
        images
            .into_iter()
            .filter_map(|node| match node {
                GraphNode::Image { image, .. } => Some(*image),
                GraphNode::Quad { .. } | GraphNode::TextureImage { .. } => None,
            })
            .collect()
    }

    #[must_use]
    pub fn ordered_content_layers(&self) -> Vec<GpuLayer> {
        let mut nodes = self.nodes.clone();
        nodes.sort_by(node_order);
        nodes.iter().map(GraphNode::layer).collect()
    }

    const fn viewport(&self) -> PixelRect {
        PixelRect::new(0, 0, self.width, self.height)
    }

    fn allocate_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        sequence
    }

    #[expect(
        clippy::too_many_lines,
        reason = "graph ordering, clipping, bounded texture planning, and instance batching remain one audited transaction"
    )]
    fn prepare(
        &self,
        instance_budget_bytes: usize,
        image_budget_bytes: usize,
    ) -> Result<PreparedGraph, GpuLayerError> {
        let viewport = self.viewport();
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(self.nodes.len()).map_err(|error| {
            GpuLayerError::message(format!("reserve bounded GPU graph ordering: {error}"))
        })?;
        nodes.extend(
            self.nodes
                .iter()
                .filter_map(|node| node.rect().intersection(viewport).map(|rect| (node, rect))),
        );
        nodes.sort_by(|(left, _), (right, _)| node_order(left, right));

        let maximum_instance_bytes = nodes
            .len()
            .checked_mul(INSTANCE_SIZE)
            .ok_or_else(|| GpuLayerError::message("GPU graph instance byte length overflow"))?;
        if maximum_instance_bytes > instance_budget_bytes {
            return Err(GpuLayerError::message(format!(
                "GPU graph can require {maximum_instance_bytes} instance bytes, exceeding the {instance_budget_bytes}-byte budget"
            )));
        }
        let mut textures = Vec::<PlannedTexture>::new();
        let mut texture_candidates = HashMap::<TextureIdentity, usize>::new();
        let mut texture_indices = HashMap::<u64, usize>::new();
        let mut unique_image_bytes = 0_usize;
        let mut texture_materializations = 0_u64;
        for (node, _) in &nodes {
            let GraphNode::TextureImage {
                sequence,
                texture,
                plan,
                ..
            } = node
            else {
                continue;
            };
            let retained_bytes =
                texture_retained_bytes(texture_byte_len(plan.width, plan.height)?)?;
            if retained_bytes > image_budget_bytes {
                return Err(GpuLayerError::message(format!(
                    "GPU image draw requires {retained_bytes} retained bytes, exceeding the {image_budget_bytes}-byte budget"
                )));
            }
            let identity = if let Some(texture) = texture.clone() {
                texture
            } else {
                texture_materializations = texture_materializations.saturating_add(1);
                texture_identity(plan)?
            };
            let texture_index = if let Some(index) = texture_candidates.get(&identity).copied() {
                index
            } else {
                unique_image_bytes =
                    unique_image_bytes
                        .checked_add(retained_bytes)
                        .ok_or_else(|| {
                            GpuLayerError::message("GPU image frame byte length overflow")
                        })?;
                if unique_image_bytes > image_budget_bytes {
                    return Err(GpuLayerError::message(format!(
                        "unique GPU image draws require {unique_image_bytes} retained bytes, exceeding the {image_budget_bytes}-byte budget"
                    )));
                }
                let index = textures.len();
                texture_candidates.insert(identity.clone(), index);
                textures.push(PlannedTexture { identity });
                index
            };
            texture_indices.insert(*sequence, texture_index);
        }

        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(maximum_instance_bytes)
            .map_err(|error| {
                GpuLayerError::message(format!("reserve bounded GPU instances: {error}"))
            })?;
        let mut batches = Vec::<DrawBatch>::new();
        let mut instance_index = 0_u32;
        for (node, rect) in nodes {
            let (kind, color) = match node {
                GraphNode::Quad { quad, .. } => (PrimitiveKind::Quad, quad.color()),
                GraphNode::Image { image, .. } => (PrimitiveKind::Image, image.color()),
                GraphNode::TextureImage { sequence, .. } => {
                    let texture_index = *texture_indices.get(sequence).ok_or_else(|| {
                        GpuLayerError::message("missing prepared GPU image texture index")
                    })?;
                    (PrimitiveKind::TextureImage(texture_index), [u8::MAX; 4])
                }
            };
            encode_instance(rect, color, &mut bytes);
            let layer = node.layer();
            if let Some(batch) = batches
                .last_mut()
                .filter(|batch| batch.kind == kind && batch.layer == layer)
            {
                batch.instances.end = batch.instances.end.saturating_add(1);
            } else {
                batches.push(DrawBatch {
                    layer,
                    kind,
                    instances: instance_index..instance_index.saturating_add(1),
                });
            }
            instance_index = instance_index.saturating_add(1);
        }
        Ok(PreparedGraph {
            bytes,
            batches,
            textures,
            texture_materializations,
        })
    }
}

fn texture_identity(plan: &ImageDrawPlan) -> Result<TextureIdentity, GpuLayerError> {
    let byte_len = texture_byte_len(plan.width, plan.height)?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(byte_len).map_err(|error| {
        GpuLayerError::message(format!(
            "reserve bounded GPU image materialization: {error}"
        ))
    })?;
    for y in 0..plan.height {
        for x in 0..plan.width {
            pixels.extend_from_slice(&image_draw_pixel(plan, x, y));
        }
    }
    Ok(TextureIdentity::from_rgba(
        plan.width,
        plan.height,
        pixels.into(),
    ))
}

fn node_order(left: &GraphNode, right: &GraphNode) -> Ordering {
    graph_node_order_key(left).cmp(&graph_node_order_key(right))
}

fn graph_node_order_key(node: &GraphNode) -> GraphNodeOrderKey {
    let (kind, z_index, id_group, kitty_image_id, parent_index, fragment_index) = match node {
        GraphNode::Quad { .. } => (GraphNodeKind::Primitive, 0, 0, 0, 0, 0),
        GraphNode::Image { image, .. } => (
            GraphNodeKind::Whole,
            image.z_index(),
            u8::from(image.kitty_id().is_none()),
            image.kitty_id().unwrap_or_default(),
            0,
            0,
        ),
        GraphNode::TextureImage { plan, .. } => match plan.tie_policy {
            ImageTiePolicy::Whole => (
                GraphNodeKind::Whole,
                plan.z_index,
                u8::from(plan.kitty_image_id.is_none()),
                plan.kitty_image_id.unwrap_or_default(),
                0,
                0,
            ),
            ImageTiePolicy::Fragment => (
                GraphNodeKind::Fragment,
                plan.z_index,
                u8::from(plan.kitty_image_id.is_some()),
                plan.kitty_image_id.unwrap_or_default(),
                plan.parent_index,
                plan.fragment_index,
            ),
        },
    };
    GraphNodeOrderKey {
        layer: node.layer().rank(),
        kind,
        z_index,
        id_group,
        kitty_image_id,
        parent_index,
        fragment_index,
        sequence: node.sequence(),
    }
}

#[derive(Clone, Debug)]
struct PreparedGraph {
    bytes: Vec<u8>,
    batches: Vec<DrawBatch>,
    textures: Vec<PlannedTexture>,
    texture_materializations: u64,
}

#[derive(Clone, Debug)]
struct PlannedTexture {
    identity: TextureIdentity,
}

#[derive(Clone, Debug)]
struct DrawBatch {
    layer: GpuLayer,
    kind: PrimitiveKind,
    instances: Range<u32>,
}

/// Most recent persistent-instance upload.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InstanceUploadMetrics {
    pub bytes_written: usize,
    pub dirty_offset: u64,
    pub capacity_bytes: usize,
    pub reallocated: bool,
}

#[derive(Debug)]
struct PersistentInstances {
    buffer: Option<wgpu::Buffer>,
    shadow: Vec<u8>,
    capacity_bytes: usize,
    budget_bytes: usize,
    metrics: InstanceUploadMetrics,
}

impl PersistentInstances {
    const fn new(budget_bytes: usize) -> Self {
        Self {
            buffer: None,
            shadow: Vec::new(),
            capacity_bytes: 0,
            budget_bytes,
            metrics: InstanceUploadMetrics {
                bytes_written: 0,
                dirty_offset: 0,
                capacity_bytes: 0,
                reallocated: false,
            },
        }
    }

    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> Result<(), GpuLayerError> {
        if bytes.len() > self.budget_bytes {
            return Err(GpuLayerError::message(format!(
                "GPU instance data requires {} bytes, exceeding the {}-byte budget",
                bytes.len(),
                self.budget_bytes
            )));
        }

        let reallocated = bytes.len() > self.capacity_bytes;
        if reallocated {
            let geometric_capacity = bytes
                .len()
                .max(MIN_INSTANCE_CAPACITY)
                .checked_next_power_of_two()
                .ok_or_else(|| GpuLayerError::message("GPU instance capacity overflow"))?;
            let aligned_budget =
                self.budget_bytes / INSTANCE_WRITE_ALIGNMENT * INSTANCE_WRITE_ALIGNMENT;
            let capacity = geometric_capacity.min(aligned_budget);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rterm-terminal-layer-instances"),
                size: capacity as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.capacity_bytes = capacity;
        }

        let dirty = if reallocated {
            (!bytes.is_empty()).then_some(0..bytes.len())
        } else {
            dirty_range(&self.shadow, bytes)
        };
        debug_assert_eq!(wgpu::COPY_BUFFER_ALIGNMENT, 4_u64);
        let aligned = dirty.map(|range| {
            let start = range.start / INSTANCE_WRITE_ALIGNMENT * INSTANCE_WRITE_ALIGNMENT;
            let end = range
                .end
                .div_ceil(INSTANCE_WRITE_ALIGNMENT)
                .saturating_mul(INSTANCE_WRITE_ALIGNMENT)
                .min(bytes.len());
            start..end
        });
        if let Some(range) = aligned.as_ref().filter(|range| !range.is_empty()) {
            let buffer = self.buffer.as_ref().ok_or_else(|| {
                GpuLayerError::message("persistent instance buffer was not allocated")
            })?;
            queue.write_buffer(buffer, range.start as u64, &bytes[range.clone()]);
        }
        self.shadow.clear();
        self.shadow.extend_from_slice(bytes);
        self.metrics = InstanceUploadMetrics {
            bytes_written: aligned.as_ref().map_or(0, Range::len),
            dirty_offset: aligned.as_ref().map_or(0, |range| range.start as u64),
            capacity_bytes: self.capacity_bytes,
            reallocated,
        };
        Ok(())
    }
}

fn dirty_range(previous: &[u8], next: &[u8]) -> Option<Range<usize>> {
    let common = previous.len().min(next.len());
    let first_difference = previous
        .iter()
        .zip(next)
        .position(|(left, right)| left != right);
    let first = first_difference.or_else(|| (previous.len() != next.len()).then_some(common))?;
    let last_changed_in_common = (first..common)
        .rev()
        .find(|index| previous[*index] != next[*index])
        .map_or(first, |index| index.saturating_add(1));
    let end = if next.len() > previous.len() {
        next.len()
    } else {
        last_changed_in_common
    };
    Some(first..end)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextureCacheMetrics {
    pub entries: usize,
    pub retained_bytes: usize,
    pub budget_bytes: usize,
    pub uploads: u64,
    pub evictions: u64,
    pub materializations: u64,
}

#[derive(Debug)]
struct CachedTexture {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    retained_bytes: usize,
    last_used: u64,
}

#[derive(Debug)]
struct TextureCache {
    entries: HashMap<TextureIdentity, CachedTexture>,
    budget_bytes: usize,
    retained_bytes: usize,
    clock: u64,
    uploads: u64,
    evictions: u64,
}

impl TextureCache {
    fn new(budget_bytes: usize) -> Result<Self, GpuLayerError> {
        if budget_bytes < 8 {
            return Err(GpuLayerError::message(
                "GPU image texture budget must be at least 8 retained bytes",
            ));
        }
        Ok(Self {
            entries: HashMap::new(),
            budget_bytes,
            retained_bytes: 0,
            clock: 0,
            uploads: 0,
            evictions: 0,
        })
    }

    fn validate(
        &self,
        device: &wgpu::Device,
        textures: &[PlannedTexture],
    ) -> Result<(), GpuLayerError> {
        let max_dimension = device.limits().max_texture_dimension_2d;
        let requested_bytes = textures.iter().try_fold(0_usize, |total, texture| {
            if texture.identity.width == 0
                || texture.identity.height == 0
                || texture.identity.width > max_dimension
                || texture.identity.height > max_dimension
            {
                return Err(GpuLayerError::message(format!(
                    "GPU image texture {}x{} exceeds device limit {max_dimension}",
                    texture.identity.width, texture.identity.height
                )));
            }
            let expected = texture_byte_len(texture.identity.width, texture.identity.height)?;
            if expected != texture.identity.pixels.len() {
                return Err(GpuLayerError::message(format!(
                    "planned image {}x{} has {} bytes, expected {expected}",
                    texture.identity.width,
                    texture.identity.height,
                    texture.identity.pixels.len()
                )));
            }
            total
                .checked_add(texture_retained_bytes(expected)?)
                .ok_or_else(|| GpuLayerError::message("GPU image texture budget overflow"))
        })?;
        if requested_bytes > self.budget_bytes {
            return Err(GpuLayerError::message(format!(
                "frame image textures require {requested_bytes} bytes, exceeding the {}-byte budget",
                self.budget_bytes
            )));
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "cache admission, LRU eviction, device validation, upload, and accounting remain one audited transaction"
    )]
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        textures: &mut [PlannedTexture],
    ) -> Result<(), GpuLayerError> {
        self.validate(device, textures)?;
        let requested = textures
            .iter()
            .map(|texture| texture.identity.clone())
            .collect::<HashSet<_>>();

        let missing_bytes = textures
            .iter()
            .filter(|texture| !self.entries.contains_key(&texture.identity))
            .try_fold(0_usize, |total, texture| {
                total
                    .checked_add(texture_retained_bytes(texture_byte_len(
                        texture.identity.width,
                        texture.identity.height,
                    )?)?)
                    .ok_or_else(|| GpuLayerError::message("GPU image cache size overflow"))
            })?;
        while self.retained_bytes.saturating_add(missing_bytes) > self.budget_bytes {
            let Some(eviction_key) = self
                .entries
                .iter()
                .filter(|(key, _)| !requested.contains(key))
                .min_by_key(|(_, texture)| texture.last_used)
                .map(|(key, _)| key.clone())
            else {
                return Err(GpuLayerError::message(
                    "GPU image cache cannot satisfy the frame texture budget",
                ));
            };
            if let Some(evicted) = self.entries.remove(&eviction_key) {
                self.retained_bytes = self.retained_bytes.saturating_sub(evicted.retained_bytes);
                self.evictions = self.evictions.saturating_add(1);
            }
        }

        for planned in textures {
            self.clock = self.clock.saturating_add(1);
            if let Some(canonical) = self
                .entries
                .get_key_value(&planned.identity)
                .map(|(identity, _)| identity.clone())
            {
                let cached = self.entries.get_mut(&canonical).ok_or_else(|| {
                    GpuLayerError::message("canonical GPU texture cache key disappeared")
                })?;
                cached.last_used = self.clock;
                planned.identity = canonical;
                continue;
            }
            let byte_len = texture_byte_len(planned.identity.width, planned.identity.height)?;
            let retained_bytes = texture_retained_bytes(byte_len)?;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("rterm-terminal-inline-image"),
                size: wgpu::Extent3d {
                    width: planned.identity.width,
                    height: planned.identity.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &planned.identity.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(planned.identity.width.saturating_mul(4)),
                    rows_per_image: Some(planned.identity.height),
                },
                wgpu::Extent3d {
                    width: planned.identity.width,
                    height: planned.identity.height,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("rterm-terminal-inline-image-bind-group"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });
            self.entries.insert(
                planned.identity.clone(),
                CachedTexture {
                    _texture: texture,
                    bind_group,
                    retained_bytes,
                    last_used: self.clock,
                },
            );
            self.retained_bytes = self.retained_bytes.saturating_add(retained_bytes);
            self.uploads = self.uploads.saturating_add(1);
        }
        Ok(())
    }

    fn metrics(&self) -> TextureCacheMetrics {
        TextureCacheMetrics {
            entries: self.entries.len(),
            retained_bytes: self.retained_bytes,
            budget_bytes: self.budget_bytes,
            uploads: self.uploads,
            evictions: self.evictions,
            materializations: 0,
        }
    }
}

fn texture_byte_len(width: u32, height: u32) -> Result<usize, GpuLayerError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| GpuLayerError::message("GPU image texture byte length overflow"))
}

fn texture_retained_bytes(gpu_bytes: usize) -> Result<usize, GpuLayerError> {
    gpu_bytes
        .checked_mul(2)
        .ok_or_else(|| GpuLayerError::message("GPU image retained byte length overflow"))
}

#[derive(Debug)]
struct LayerPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl LayerPipeline {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        blend: Option<wgpu::BlendState>,
        label: &'static str,
    ) -> Self {
        let attributes = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 16,
                shader_location: 1,
            },
        ];
        let buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: INSTANCE_STRIDE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        })];
        let targets = [Some(wgpu::ColorTargetState {
            format,
            blend,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline }
    }
}

impl GpuImagePipeline {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        viewport_bind_group_layout: &wgpu::BindGroupLayout,
        shader_source: &str,
    ) -> Result<Self, GpuLayerError> {
        let internal_scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let out_of_memory_scope = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rterm-terminal-image-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source)),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rterm-terminal-image-bind-group-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rterm-terminal-image-pipeline-layout"),
            bind_group_layouts: &[Some(viewport_bind_group_layout), Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = LayerPipeline::new(
            device,
            format,
            &pipeline_layout,
            &shader,
            None,
            "rterm-terminal-image-pipeline",
        )
        .pipeline;
        let candidate = Self {
            pipeline,
            bind_group_layout,
        };
        let validation_error = pollster::block_on(validation_scope.pop());
        let out_of_memory_error = pollster::block_on(out_of_memory_scope.pop());
        let internal_error = pollster::block_on(internal_scope.pop());
        let errors = [
            ("out-of-memory", out_of_memory_error),
            ("internal", internal_error),
            ("validation", validation_error),
        ]
        .into_iter()
        .filter_map(|(kind, error)| error.map(|error| format!("{kind}: {error}")))
        .collect::<Vec<_>>();
        if !errors.is_empty() {
            return Err(GpuLayerError::message(format!(
                "create GPU image pipeline: {}",
                errors.join("; ")
            )));
        }
        Ok(candidate)
    }
}

/// Persistent instanced renderer for non-text terminal layers.
#[derive(Debug)]
pub struct GpuLayerRenderer {
    generation: GpuContextGeneration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    viewport: wgpu::Buffer,
    quads: LayerPipeline,
    images: Option<GpuImagePipeline>,
    instances: PersistentInstances,
    texture_cache: TextureCache,
    readback_budget_bytes: usize,
    prepared_batches: Vec<DrawBatch>,
    prepared_textures: Vec<TextureIdentity>,
    texture_materializations: u64,
    base_text_renderer_materializations: u64,
    cursor_text_renderer_materializations: u64,
    accounted_text_materializations: GpuTextOwnerMaterialization,
    prepared_size: (u32, u32),
    text: Option<GpuText>,
    #[cfg(any(test, debug_assertions))]
    image_pipeline_creation_failure_injections: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReadbackLayout {
    unpadded_bytes_per_row_usize: usize,
    padded_bytes_per_row: u32,
    padded_bytes_per_row_usize: usize,
    readback_size: u64,
    output_len: usize,
}

impl GpuLayerRenderer {
    /// Creates an RGBA renderer for a headless context without requiring the
    /// caller to depend directly on wgpu's texture-format type.
    ///
    /// # Errors
    ///
    /// Returns an error when the budget cannot hold one instance.
    pub fn new_headless(
        context: &GpuContext,
        instance_budget_bytes: usize,
    ) -> Result<Self, GpuLayerError> {
        Self::new(
            context,
            wgpu::TextureFormat::Rgba8Unorm,
            instance_budget_bytes,
        )
    }

    /// Creates pipelines with a strict upper bound for retained instance bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the budget cannot hold one instance.
    pub fn new(
        context: &GpuContext,
        format: wgpu::TextureFormat,
        instance_budget_bytes: usize,
    ) -> Result<Self, GpuLayerError> {
        Self::new_with_budgets(
            context,
            format,
            instance_budget_bytes,
            DEFAULT_GPU_IMAGE_BYTE_BUDGET,
        )
    }

    /// Creates pipelines with independent retained instance and image budgets.
    ///
    /// # Errors
    ///
    /// Returns an error when either budget is too small.
    pub fn new_with_budgets(
        context: &GpuContext,
        format: wgpu::TextureFormat,
        instance_budget_bytes: usize,
        image_budget_bytes: usize,
    ) -> Result<Self, GpuLayerError> {
        Self::new_with_all_budgets(
            context,
            format,
            instance_budget_bytes,
            image_budget_bytes,
            DEFAULT_GPU_READBACK_BYTE_BUDGET,
        )
    }

    /// Creates pipelines with independent instance, image, and readback budgets.
    ///
    /// # Errors
    ///
    /// Returns an error when a budget is too small for its minimum resource.
    pub fn new_with_all_budgets(
        context: &GpuContext,
        format: wgpu::TextureFormat,
        instance_budget_bytes: usize,
        image_budget_bytes: usize,
        readback_budget_bytes: usize,
    ) -> Result<Self, GpuLayerError> {
        let device = context.device();
        let queue = context.queue();
        if instance_budget_bytes < MIN_INSTANCE_CAPACITY {
            return Err(GpuLayerError::message(format!(
                "GPU instance budget must be at least {MIN_INSTANCE_CAPACITY} bytes"
            )));
        }
        if readback_budget_bytes < 4 {
            return Err(GpuLayerError::message(
                "GPU readback budget must be at least 4 bytes",
            ));
        }
        let device_buffer_limit =
            usize::try_from(device.limits().max_buffer_size).unwrap_or(usize::MAX);
        let instance_budget_bytes = instance_budget_bytes.min(device_buffer_limit);
        if instance_budget_bytes < MIN_INSTANCE_CAPACITY {
            return Err(GpuLayerError::message(
                "GPU device cannot allocate the minimum instance buffer",
            ));
        }
        let viewport = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rterm-terminal-layer-viewport"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rterm-terminal-layer-bind-group-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(8),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rterm-terminal-layer-bind-group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport.as_entire_binding(),
            }],
        });
        let quad_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rterm-terminal-quad-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(QUAD_SHADER)),
        });
        let quad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rterm-terminal-quad-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        Ok(Self {
            generation: context.generation(),
            device: device.clone(),
            queue: queue.clone(),
            format,
            bind_group,
            bind_group_layout,
            viewport,
            quads: LayerPipeline::new(
                device,
                format,
                &quad_pipeline_layout,
                &quad_shader,
                Some(wgpu::BlendState::ALPHA_BLENDING),
                "rterm-terminal-quad-pipeline",
            ),
            images: None,
            instances: PersistentInstances::new(instance_budget_bytes),
            texture_cache: TextureCache::new(image_budget_bytes)?,
            readback_budget_bytes,
            prepared_batches: Vec::new(),
            prepared_textures: Vec::new(),
            texture_materializations: 0,
            base_text_renderer_materializations: 0,
            cursor_text_renderer_materializations: 0,
            accounted_text_materializations: GpuTextOwnerMaterialization::default(),
            prepared_size: (0, 0),
            text: None,
            #[cfg(any(test, debug_assertions))]
            image_pipeline_creation_failure_injections: 0,
        })
    }

    #[cfg(any(test, debug_assertions))]
    #[doc(hidden)]
    pub fn inject_image_pipeline_creation_failure_for_test(&mut self) {
        self.image_pipeline_creation_failure_injections = self
            .image_pipeline_creation_failure_injections
            .saturating_add(1);
    }

    /// Installs the opt-in direct GPU text backend. This does not change the
    /// application's compatibility renderer selection.
    ///
    /// # Errors
    ///
    /// Returns an error when the physical atlas cannot fit its configured
    /// minimum allocation.
    pub fn enable_text(
        &mut self,
        catalog: FontCatalog,
        font_config: FontConfig,
        config: GpuTextConfig,
    ) -> Result<(), GpuLayerError> {
        let text = GpuText::new(
            self.generation,
            self.device.clone(),
            self.queue.clone(),
            self.format,
            catalog,
            font_config,
            config,
        )?;
        self.accounted_text_materializations = GpuTextOwnerMaterialization::default();
        self.text = Some(text);
        self.reconcile_text_renderer_materializations();
        Ok(())
    }

    fn reconcile_text_renderer_materializations(&mut self) {
        let current = self.text.as_ref().map_or_else(
            GpuTextOwnerMaterialization::default,
            GpuText::owner_materialization,
        );
        self.base_text_renderer_materializations =
            self.base_text_renderer_materializations.saturating_add(
                current
                    .base_renderer_count
                    .saturating_sub(self.accounted_text_materializations.base_renderer_count),
            );
        self.cursor_text_renderer_materializations =
            self.cursor_text_renderer_materializations.saturating_add(
                current
                    .cursor_renderer_count
                    .saturating_sub(self.accounted_text_materializations.cursor_renderer_count),
            );
        self.accounted_text_materializations = current;
    }

    /// Prepares terminal glyphs from the authoritative shaped row and raster
    /// caches, without asking glyphon to shape the text a second time.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid geometry or scale, shaping failure,
    /// identifier exhaustion, or an atlas/payload budget violation.
    pub fn prepare_text(
        &mut self,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        damage: &[DamageRegion],
        paint: &TextPaintConfig,
        dpi_scale: f32,
        zoom: f32,
    ) -> Result<GpuTextPrepareReport, GpuLayerError> {
        let result = self
            .text
            .as_mut()
            .ok_or_else(|| GpuLayerError::message("GPU text is not enabled"))?
            .prepare(snapshot, geometry, damage, paint, dpi_scale, zoom);
        self.reconcile_text_renderer_materializations();
        result
    }

    /// Prepares one complete frame only when the app's preflight generation is
    /// still current. A mismatch asks the caller to restart the whole frame.
    ///
    /// # Errors
    ///
    /// Returns the same bounded shaping and atlas errors as [`Self::prepare_text`].
    #[expect(
        clippy::too_many_arguments,
        reason = "generation joins the existing explicit whole-frame text preparation contract"
    )]
    pub fn prepare_text_for_catalog_generation(
        &mut self,
        expected_generation: u64,
        snapshot: &TerminalRenderSnapshot,
        geometry: RenderGeometry,
        damage: &[DamageRegion],
        paint: &TextPaintConfig,
        dpi_scale: f32,
        zoom: f32,
    ) -> Result<GpuTextCatalogStatus, GpuLayerError> {
        let result = self
            .text
            .as_mut()
            .ok_or_else(|| GpuLayerError::message("GPU text is not enabled"))?
            .prepare_for_catalog_generation(
                expected_generation,
                snapshot,
                geometry,
                damage,
                paint,
                dpi_scale,
                zoom,
            );
        self.reconcile_text_renderer_materializations();
        result
    }

    #[must_use]
    pub fn text_atlas_metrics(&self) -> Option<GpuTextAtlasMetrics> {
        self.text.as_ref().map(GpuText::metrics)
    }

    pub fn text_catalog_mut(&mut self) -> Option<&mut FontCatalog> {
        self.text.as_mut().map(GpuText::catalog_mut)
    }

    #[must_use]
    pub fn text_catalog_generation(&self) -> Option<u64> {
        self.text.as_ref().map(GpuText::catalog_generation)
    }

    #[must_use]
    pub fn text_cpu_font_metrics(&self) -> Option<GpuTextCpuFontMetrics> {
        self.text.as_ref().map(GpuText::cpu_font_metrics)
    }

    /// Releases CPU-side font state before a lost renderer is retained solely
    /// to defer destruction of unsafe driver GPU objects.
    pub fn retire_text_cpu_font_state(&mut self) {
        if let Some(text) = self.text.as_mut() {
            text.retire_cpu_font_state();
        }
    }

    /// Clears a partially prepared frame after app-owned font preflight grows
    /// the catalog and before the caller restarts the complete frame.
    ///
    /// # Errors
    ///
    /// Returns an error if rebuilding the empty GPU text scope fails.
    pub fn discard_prepared_text_frame(&mut self) -> Result<(), GpuLayerError> {
        let result = self
            .text
            .as_mut()
            .ok_or_else(|| GpuLayerError::message("GPU text is not enabled"))?
            .discard_prepared_frame();
        self.reconcile_text_renderer_materializations();
        result
    }

    /// Updates the persistent instance buffer, writing only its dirty range.
    ///
    /// # Errors
    ///
    /// Returns an error if graph instances exceed the configured byte budget.
    pub fn upload(&mut self, graph: &RenderGraph) -> Result<(), GpuLayerError> {
        let extended = self.graph_with_text_blocks(graph);
        let graph = extended.as_ref().unwrap_or(graph);
        let mut prepared =
            graph.prepare(self.instances.budget_bytes, self.texture_cache.budget_bytes)?;
        let texture_materializations = prepared.texture_materializations;
        self.prepare_image_resources(&mut prepared.textures)?;
        self.instances
            .upload(&self.device, &self.queue, &prepared.bytes)?;
        self.texture_materializations = self
            .texture_materializations
            .saturating_add(texture_materializations);
        self.prepared_batches = prepared.batches;
        self.prepared_textures = prepared
            .textures
            .into_iter()
            .map(|texture| texture.identity)
            .collect();
        self.prepared_size = (graph.width, graph.height);
        self.write_viewport();
        Ok(())
    }

    fn prepare_image_resources(
        &mut self,
        textures: &mut [PlannedTexture],
    ) -> Result<(), GpuLayerError> {
        if textures.is_empty() {
            return Ok(());
        }
        self.texture_cache.validate(&self.device, textures)?;
        if self.images.is_none() {
            #[cfg(any(test, debug_assertions))]
            let shader_source = if self.image_pipeline_creation_failure_injections == 0 {
                IMAGE_SHADER
            } else {
                self.image_pipeline_creation_failure_injections = self
                    .image_pipeline_creation_failure_injections
                    .saturating_sub(1);
                INVALID_IMAGE_SHADER_FOR_TEST
            };
            #[cfg(not(any(test, debug_assertions)))]
            let shader_source = IMAGE_SHADER;
            let candidate = GpuImagePipeline::new(
                &self.device,
                self.format,
                &self.bind_group_layout,
                shader_source,
            )?;
            self.images = Some(candidate);
        }
        let layout = &self
            .images
            .as_ref()
            .expect("image pipeline was materialized for a non-empty texture batch")
            .bind_group_layout;
        self.texture_cache
            .prepare(&self.device, &self.queue, layout, textures)
    }

    fn graph_with_text_blocks(&self, graph: &RenderGraph) -> Option<RenderGraph> {
        let blocks = self.text.as_ref()?.block_quads();
        if blocks.is_empty() {
            return None;
        }
        let mut extended = graph.clone();
        for block in blocks {
            extended.push_quad(*block);
        }
        Some(extended)
    }

    /// Checked compatibility entry point for callers migrating from explicit
    /// device/queue plumbing.
    ///
    /// # Errors
    ///
    /// Rejects a different device or queue before preparing or mutating state.
    pub fn upload_from(
        &mut self,
        context: &GpuContext,
        graph: &RenderGraph,
    ) -> Result<(), GpuLayerError> {
        if context.generation() != self.generation
            || context.device() != &self.device
            || context.queue() != &self.queue
        {
            return Err(GpuLayerError::message(
                "different GPU context cannot use this layer renderer",
            ));
        }
        self.upload(graph)
    }

    #[must_use]
    pub const fn upload_metrics(&self) -> InstanceUploadMetrics {
        self.instances.metrics
    }

    #[must_use]
    pub const fn instance_budget_bytes(&self) -> usize {
        self.instances.budget_bytes
    }

    #[must_use]
    pub fn texture_cache_metrics(&self) -> TextureCacheMetrics {
        let mut metrics = self.texture_cache.metrics();
        metrics.materializations = self.texture_materializations;
        metrics
    }

    /// Reports only the buffers, textures, and pipelines owned by this live
    /// renderer. The caller can merge it with its context snapshot to obtain a
    /// cumulative initialization-stage view.
    #[must_use]
    pub fn initialization_resources(&self) -> GpuInitializationResourceSnapshot {
        let instance_bytes = u64::try_from(self.instances.capacity_bytes).unwrap_or(u64::MAX);
        let texture_metrics = self.texture_cache_metrics();
        let image_texture_bytes = u64::try_from(texture_metrics.retained_bytes).unwrap_or(u64::MAX);
        let text_atlas = self.text_atlas_metrics().unwrap_or_default();
        let glyph_atlas_bytes =
            u64::try_from(text_atlas.physical_texture_bytes).unwrap_or(u64::MAX);
        let raster_cache_bytes = self.text_cpu_font_metrics().map_or(0, |metrics| {
            u64::try_from(metrics.raster_cache_bytes).unwrap_or(u64::MAX)
        });
        GpuInitializationResourceSnapshot {
            pipeline_count: 1 + u64::from(self.images.is_some()),
            pipeline_layout_count: 1 + u64::from(self.images.is_some()),
            materialized_buffer_count: 1 + u64::from(self.instances.buffer.is_some()),
            instance_buffer_bytes: instance_bytes,
            total_allocated_buffer_bytes: 8_u64.saturating_add(instance_bytes),
            total_allocated_texture_bytes: image_texture_bytes.saturating_add(glyph_atlas_bytes),
            glyph_atlas_bytes,
            raster_cache_bytes,
            image_texture_bytes,
            base_text_renderer_materialization_count: self.base_text_renderer_materializations,
            cursor_text_renderer_materialization_count: self.cursor_text_renderer_materializations,
            ..GpuInitializationResourceSnapshot::default()
        }
    }

    fn validate_readback(&self, width: u32, height: u32) -> Result<ReadbackLayout, GpuLayerError> {
        let limits = self.device.limits();
        if width > limits.max_texture_dimension_2d || height > limits.max_texture_dimension_2d {
            return Err(GpuLayerError::message(format!(
                "headless texture {width}x{height} exceeds device dimension limit {}",
                limits.max_texture_dimension_2d
            )));
        }
        let unpadded_bytes_per_row = width
            .checked_mul(4)
            .ok_or_else(|| GpuLayerError::message("readback row pitch overflow"))?;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            .checked_mul(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            .ok_or_else(|| GpuLayerError::message("aligned readback row pitch overflow"))?;
        let readback_size = u64::from(padded_bytes_per_row)
            .checked_mul(u64::from(height))
            .ok_or_else(|| GpuLayerError::message("readback buffer size overflow"))?;
        let output_size = u64::from(unpadded_bytes_per_row)
            .checked_mul(u64::from(height))
            .ok_or_else(|| GpuLayerError::message("tight readback size overflow"))?;
        let budget = u64::try_from(self.readback_budget_bytes).unwrap_or(u64::MAX);
        if readback_size > budget || output_size > budget {
            return Err(GpuLayerError::message(format!(
                "headless readback requires {readback_size} padded bytes and {output_size} output bytes, exceeding the {}-byte budget",
                self.readback_budget_bytes
            )));
        }
        if readback_size > limits.max_buffer_size {
            return Err(GpuLayerError::message(format!(
                "readback buffer {readback_size} exceeds device buffer limit {}",
                limits.max_buffer_size
            )));
        }
        Ok(ReadbackLayout {
            unpadded_bytes_per_row_usize: usize::try_from(unpadded_bytes_per_row)
                .map_err(|_| GpuLayerError::message("row pitch exceeds host address space"))?,
            padded_bytes_per_row,
            padded_bytes_per_row_usize: usize::try_from(padded_bytes_per_row)
                .map_err(|_| GpuLayerError::message("padded row exceeds host address space"))?,
            readback_size,
            output_len: usize::try_from(output_size)
                .map_err(|_| GpuLayerError::message("output exceeds host address space"))?,
        })
    }

    /// Renders the graph to an RGBA8 texture and returns tightly packed pixels.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid geometry, budget exhaustion, mapping
    /// failure, device polling failure, or a readback timeout.
    #[expect(
        clippy::too_many_lines,
        reason = "the headless oracle keeps render, padded copy, bounded polling, and unpadding in one auditable operation"
    )]
    pub fn render_headless_rgba8(
        &mut self,
        graph: &RenderGraph,
        timeout: Duration,
    ) -> Result<Vec<u8>, GpuLayerError> {
        if self.format != wgpu::TextureFormat::Rgba8Unorm {
            return Err(GpuLayerError::message(
                "headless RGBA8 readback requires Rgba8Unorm format",
            ));
        }
        if graph.width == 0 || graph.height == 0 {
            return Err(GpuLayerError::message(
                "headless render dimensions must be nonzero",
            ));
        }
        let layout = self.validate_readback(graph.width, graph.height)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(layout.output_len)
            .map_err(|error| {
                GpuLayerError::message(format!("reserve bounded GPU readback output: {error}"))
            })?;
        let extended = self.graph_with_text_blocks(graph);
        let graph = extended.as_ref().unwrap_or(graph);
        let mut prepared =
            graph.prepare(self.instances.budget_bytes, self.texture_cache.budget_bytes)?;
        let texture_materializations = prepared.texture_materializations;
        self.prepare_image_resources(&mut prepared.textures)?;
        self.instances
            .upload(&self.device, &self.queue, &prepared.bytes)?;
        self.texture_materializations = self
            .texture_materializations
            .saturating_add(texture_materializations);
        self.prepared_batches = prepared.batches;
        self.prepared_textures = prepared
            .textures
            .into_iter()
            .map(|texture| texture.identity)
            .collect();
        self.prepared_size = (graph.width, graph.height);
        self.write_viewport();

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rterm-terminal-layer-headless-target"),
            size: wgpu::Extent3d {
                width: graph.width,
                height: graph.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rterm-terminal-layer-readback"),
            size: layout.readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rterm-terminal-layer-headless-encoder"),
            });
        self.encode_render_pass(&mut encoder, &view)?;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(layout.padded_bytes_per_row),
                    rows_per_image: Some(graph.height),
                },
            },
            wgpu::Extent3d {
                width: graph.width,
                height: graph.height,
                depth_or_array_layers: 1,
            },
        );
        let submission_index = self.queue.submit([encoder.finish()]);

        let (sender, receiver) = mpsc::sync_channel(1);
        readback.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result);
        });
        let wait_timeout = timeout.min(MAX_GPU_READBACK_WAIT);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission_index),
                timeout: Some(wait_timeout),
            })
            .map_err(|error| {
                GpuLayerError::message(format!(
                    "GPU readback submission did not complete within {wait_timeout:?}: {error}"
                ))
            })?;
        let map_result = receiver.try_recv().map_err(|error| {
            GpuLayerError::message(format!(
                "GPU readback callback was not delivered after submission completion: {error}"
            ))
        })?;
        map_result.map_err(|error| {
            GpuLayerError::message(format!("GPU readback mapping failed: {error}"))
        })?;

        let mapped = match readback.get_mapped_range(..) {
            Ok(mapped) => mapped,
            Err(error) => {
                readback.unmap();
                return Err(GpuLayerError::message(format!(
                    "get mapped readback: {error}"
                )));
            }
        };
        for row in mapped.chunks_exact(layout.padded_bytes_per_row_usize) {
            output.extend_from_slice(&row[..layout.unpadded_bytes_per_row_usize]);
        }
        drop(mapped);
        readback.unmap();
        Ok(output)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "wgpu viewport uniforms are f32 and validated GPU dimensions are exactly representable"
    )]
    fn write_viewport(&self) {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&(self.prepared_size.0 as f32).to_ne_bytes());
        bytes.extend_from_slice(&(self.prepared_size.1 as f32).to_ne_bytes());
        self.queue.write_buffer(&self.viewport, 0, &bytes);
    }

    pub(crate) fn encode_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), GpuLayerError> {
        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rterm-terminal-layer-render-pass"),
            color_attachments: &attachments,
            ..wgpu::RenderPassDescriptor::default()
        });
        pass.set_bind_group(0, &self.bind_group, &[]);
        if let Some(buffer) = self.instances.buffer.as_ref() {
            pass.set_vertex_buffer(0, buffer.slice(..));
        }
        let glyph_rank = GpuLayer::Glyph.rank();
        let split = self
            .prepared_batches
            .partition_point(|batch| batch.layer.rank() <= glyph_rank);
        let cursor_split = self
            .prepared_batches
            .partition_point(|batch| batch.layer.rank() <= GpuLayer::Cursor.rank());
        for batch in &self.prepared_batches[..split] {
            self.draw_batch(&mut pass, batch)?;
        }
        if let Some(text) = self.text.as_ref() {
            text.render(&mut pass)?;
        }
        for batch in &self.prepared_batches[split..cursor_split] {
            self.draw_batch(&mut pass, batch)?;
        }
        if let Some(text) = self.text.as_ref() {
            text.render_cursor(&mut pass)?;
        }
        for batch in &self.prepared_batches[cursor_split..] {
            self.draw_batch(&mut pass, batch)?;
        }
        Ok(())
    }

    fn draw_batch(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        batch: &DrawBatch,
    ) -> Result<(), GpuLayerError> {
        pass.set_bind_group(0, &self.bind_group, &[]);
        if let Some(buffer) = self.instances.buffer.as_ref() {
            pass.set_vertex_buffer(0, buffer.slice(..));
        }
        match batch.kind {
            PrimitiveKind::Quad | PrimitiveKind::Image => {
                pass.set_pipeline(&self.quads.pipeline);
            }
            PrimitiveKind::TextureImage(index) => {
                let identity = self.prepared_textures.get(index).ok_or_else(|| {
                    GpuLayerError::message("prepared image texture index is absent from the frame")
                })?;
                let cached = self.texture_cache.entries.get(identity).ok_or_else(|| {
                    GpuLayerError::message("prepared image texture is absent from cache")
                })?;
                let images = self.images.as_ref().ok_or_else(|| {
                    GpuLayerError::message("prepared image pipeline is absent from the renderer")
                })?;
                pass.set_pipeline(&images.pipeline);
                pass.set_bind_group(1, &cached.bind_group, &[]);
            }
        }
        pass.draw(0..6, batch.instances.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc, time::Duration};

    use crate::gpu::{GpuContextOptions, ImageProtocol};
    use rterm_render_cpu::{DecodedImage, ImageDrawPlan, ImageTiePolicy};

    use super::{
        GpuContext, GpuImage, GpuLayer, GpuLayerRenderer, GraphNode, PixelRect, RenderGraph,
        TEXTURE_IDENTITY_EQ_CALLS, TextureIdentity, dirty_range, node_order,
    };

    fn one_pixel_plan(x: u32, pixels: [u8; 4], stable_order: usize) -> Arc<ImageDrawPlan> {
        Arc::new(ImageDrawPlan {
            destination_x: x,
            destination_y: 0,
            width: 1,
            height: 1,
            decoded: Arc::new(DecodedImage {
                width: 1,
                height: 1,
                pixels: Arc::from(pixels),
            }),
            sample_source_x: 0,
            sample_source_y: 0,
            sample_target_x: 0,
            sample_target_y: 0,
            sample_source_width: 1,
            sample_source_height: 1,
            sample_destination_width: 1,
            sample_destination_height: 1,
            z_index: 0,
            kitty_image_id: None,
            parent_index: stable_order,
            fragment_index: 0,
            tie_policy: ImageTiePolicy::Whole,
            stable_order,
        })
    }

    #[test]
    fn dirty_range_ignores_equal_prefix_and_suffix() {
        assert_eq!(dirty_range(&[1, 2, 3, 4], &[1, 8, 9, 4]), Some(1..3));
        assert_eq!(dirty_range(&[1, 2], &[1, 2]), None);
        assert_eq!(dirty_range(&[1], &[1, 2]), Some(1..2));
    }

    #[test]
    fn dirty_range_compares_offsets_instead_of_matching_a_shifted_shrink_tail() {
        assert_eq!(dirty_range(&[1, 9, 2], &[1, 2]), Some(1..2));
    }

    #[test]
    fn graph_node_order_is_transitive_across_whole_fragment_and_legacy_images() {
        fn texture_node(
            sequence: u64,
            tie_policy: ImageTiePolicy,
            kitty_image_id: Option<u32>,
        ) -> GraphNode {
            let pixels: Arc<[u8]> =
                Arc::from([u8::try_from(sequence).unwrap_or_default(), 0, 0, 255]);
            let plan = Arc::new(ImageDrawPlan {
                destination_x: 0,
                destination_y: 0,
                width: 1,
                height: 1,
                decoded: Arc::new(DecodedImage {
                    width: 1,
                    height: 1,
                    pixels: Arc::clone(&pixels),
                }),
                sample_source_x: 0,
                sample_source_y: 0,
                sample_target_x: 0,
                sample_target_y: 0,
                sample_source_width: 1,
                sample_source_height: 1,
                sample_destination_width: 1,
                sample_destination_height: 1,
                z_index: 0,
                kitty_image_id,
                parent_index: usize::try_from(sequence).unwrap_or_default(),
                fragment_index: 0,
                tie_policy,
                stable_order: usize::try_from(sequence).unwrap_or_default(),
            });
            GraphNode::TextureImage {
                sequence,
                layer: GpuLayer::PositiveImage,
                texture: Some(TextureIdentity {
                    digest: sequence,
                    width: 1,
                    height: 1,
                    pixels,
                }),
                plan,
            }
        }

        let fragment = texture_node(0, ImageTiePolicy::Fragment, Some(1));
        let legacy = GraphNode::Image {
            sequence: 1,
            image: GpuImage::new(
                ImageProtocol::Iterm,
                0,
                PixelRect::new(0, 0, 1, 1),
                [0, 0, 0, 255],
            ),
        };
        let whole = texture_node(2, ImageTiePolicy::Whole, None);
        let permutations = [
            [fragment.clone(), legacy.clone(), whole.clone()],
            [fragment.clone(), whole.clone(), legacy.clone()],
            [legacy.clone(), fragment.clone(), whole.clone()],
            [legacy.clone(), whole.clone(), fragment.clone()],
            [whole.clone(), fragment.clone(), legacy.clone()],
            [whole, legacy, fragment],
        ];
        for mut nodes in permutations {
            nodes.sort_by(node_order);
            assert_eq!(
                nodes.map(|node| node.sequence()),
                [1, 2, 0],
                "all insertion permutations must produce one total order"
            );
        }
    }

    #[test]
    fn exact_texture_identity_survives_an_artificial_digest_collision() {
        let red = one_pixel_plan(0, [255, 0, 0, 255], 0);
        let green = one_pixel_plan(1, [0, 255, 0, 255], 1);
        let colliding_digest = 7;
        let graph = RenderGraph {
            width: 2,
            height: 1,
            nodes: vec![
                GraphNode::TextureImage {
                    sequence: 0,
                    layer: GpuLayer::PositiveImage,
                    texture: Some(TextureIdentity {
                        digest: colliding_digest,
                        width: 1,
                        height: 1,
                        pixels: red.decoded.pixels.clone(),
                    }),
                    plan: red,
                },
                GraphNode::TextureImage {
                    sequence: 1,
                    layer: GpuLayer::PositiveImage,
                    texture: Some(TextureIdentity {
                        digest: colliding_digest,
                        width: 1,
                        height: 1,
                        pixels: green.decoded.pixels.clone(),
                    }),
                    plan: green,
                },
            ],
            next_sequence: 2,
        };
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256)
            .expect("renderer");

        assert_eq!(
            renderer
                .render_headless_rgba8(&graph, Duration::from_secs(5))
                .expect("collision readback"),
            [255, 0, 0, 255, 0, 255, 0, 255]
        );
        assert_eq!(renderer.texture_cache_metrics().entries, 2);
        assert_eq!(renderer.texture_cache_metrics().uploads, 2);
    }

    #[test]
    fn texture_identity_hash_and_equality_share_the_digest_seam() {
        let pixels: Arc<[u8]> = Arc::from([255, 0, 0, 255]);
        let first = TextureIdentity {
            digest: 1,
            width: 1,
            height: 1,
            pixels: pixels.clone(),
        };
        let second = TextureIdentity {
            digest: 2,
            width: 1,
            height: 1,
            pixels,
        };

        assert_ne!(first, second);
        assert_eq!(HashSet::from([first, second]).len(), 2);
    }

    #[test]
    fn candidate_texture_dedup_has_near_linear_exact_comparisons() {
        const CANDIDATES: usize = 256;
        let nodes = (0..CANDIDATES)
            .map(|index| {
                let byte = u8::try_from(index % 251).expect("bounded byte");
                GraphNode::TextureImage {
                    sequence: u64::try_from(index).expect("bounded sequence"),
                    layer: GpuLayer::PositiveImage,
                    texture: Some(TextureIdentity {
                        digest: u64::try_from(index).expect("bounded digest"),
                        width: 1,
                        height: 1,
                        pixels: Arc::from([byte, 0, 0, 255]),
                    }),
                    plan: one_pixel_plan(
                        u32::try_from(index).expect("bounded x"),
                        [byte, 0, 0, 255],
                        index,
                    ),
                }
            })
            .collect();
        let graph = RenderGraph {
            width: u32::try_from(CANDIDATES).expect("bounded width"),
            height: 1,
            nodes,
            next_sequence: u64::try_from(CANDIDATES).expect("bounded sequence"),
        };

        TEXTURE_IDENTITY_EQ_CALLS.with(|calls| calls.set(0));
        let prepared = graph.prepare(64 * 1024, 64 * 1024).expect("bounded graph");

        assert_eq!(prepared.textures.len(), CANDIDATES);
        let comparisons = TEXTURE_IDENTITY_EQ_CALLS.with(std::cell::Cell::get);
        assert!(
            comparisons < CANDIDATES * 4,
            "candidate dedup must not scan every prior texture payload"
        );
    }

    #[test]
    fn cache_hits_replace_fresh_frame_pixels_with_the_canonical_allocation() {
        let plan = one_pixel_plan(0, [255, 0, 0, 255], 0);
        let graph = RenderGraph {
            width: 1,
            height: 1,
            nodes: vec![GraphNode::TextureImage {
                sequence: 0,
                layer: GpuLayer::PositiveImage,
                texture: None,
                plan,
            }],
            next_sequence: 1,
        };
        let context = pollster::block_on(GpuContext::new_headless(GpuContextOptions::default()))
            .expect("headless adapter");
        let mut renderer = GpuLayerRenderer::new(&context, wgpu::TextureFormat::Rgba8Unorm, 256)
            .expect("renderer");

        renderer.upload(&graph).expect("first frame");
        let canonical = renderer.prepared_textures[0].pixels.clone();
        assert_eq!(renderer.texture_cache_metrics().retained_bytes, 8);
        renderer.upload(&graph).expect("second frame");

        assert!(Arc::ptr_eq(
            &canonical,
            &renderer.prepared_textures[0].pixels
        ));
        assert_eq!(renderer.texture_cache_metrics().entries, 1);
        assert_eq!(renderer.texture_cache_metrics().retained_bytes, 8);
        assert_eq!(renderer.texture_cache_metrics().uploads, 1);
    }
}
