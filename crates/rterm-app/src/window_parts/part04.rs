fn lua_inline_string_literal_value_and_len(value: &str) -> Option<(String, usize)> {
    let value = value.trim_start();
    let literal = lua_quoted_string_literal_from_query(value)
        .or_else(|| lua_long_bracket_literal_from_query(value))?;
    let parsed = if literal.starts_with('[') {
        parse_lua_long_bracket_query_text(literal)?
    } else {
        parse_lua_quoted_query_text(literal)?
    };
    Some((parsed, literal.len()))
}

enum NativeColorSchemeLuaSource<'a> {
    Table {
        colors: &'a str,
        variable: Option<NativeLoadSchemeVariableReference>,
        entry_mutation: Option<NativeColorSchemeEntryVariableReference>,
    },
    LoadScheme {
        path: String,
        variable: Option<NativeLoadSchemeVariableReference>,
        entry_mutation: Option<NativeColorSchemeEntryVariableReference>,
    },
    Builtin {
        name: String,
        variable: Option<NativeLoadSchemeVariableReference>,
        entry_mutation: Option<NativeColorSchemeEntryVariableReference>,
    },
    DefaultColors {
        variable: Option<NativeLoadSchemeVariableReference>,
        entry_mutation: Option<NativeColorSchemeEntryVariableReference>,
    },
}

impl NativeColorSchemeLuaSource<'_> {
    fn with_entry_mutation(
        mut self,
        entry_mutation: NativeColorSchemeEntryVariableReference,
    ) -> Self {
        match &mut self {
            Self::Table {
                entry_mutation: slot,
                ..
            }
            | Self::LoadScheme {
                entry_mutation: slot,
                ..
            }
            | Self::Builtin {
                entry_mutation: slot,
                ..
            }
            | Self::DefaultColors {
                entry_mutation: slot,
                ..
            } => {
                *slot = Some(entry_mutation);
            }
        }
        self
    }
}

#[derive(Debug, Clone)]
struct NativeLoadSchemeVariableReference {
    name: String,
    mutation_max_start: usize,
    mutation_events: Vec<LuaPaletteMutationEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LuaPaletteMutationEvent {
    statement: LuaLogicalStatement,
}

#[derive(Debug, Clone)]
struct NativeColorSchemeEntryVariableReference {
    variable: String,
    mutation_start: usize,
    mutation_max_start: usize,
}

#[derive(Clone, Copy)]
enum NativeColorSchemeEntryMutationTarget<'a> {
    Config { receiver: &'a str },
    Variable { variable: &'a str },
}

fn apply_lua_color_scheme_source_overrides(
    config: &str,
    color_scheme: &str,
    source: NativeColorSchemeLuaSource<'_>,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    match source {
        NativeColorSchemeLuaSource::Table {
            colors,
            variable,
            entry_mutation,
        } => {
            let static_source = Some(LuaStaticSource {
                source: config,
                max_start: lua_source_slice_start_offset(config, colors)?,
            });
            let mut parsed = apply_lua_colors_table_overrides(static_source, colors, overrides)?;
            if let Some(variable) = variable.as_ref() {
                parsed |= apply_lua_color_variable_mutation_overrides(config, variable, overrides)?;
            }
            if let Some(entry_mutation) = entry_mutation.as_ref() {
                parsed |= apply_lua_color_scheme_entry_mutation_overrides(
                    config,
                    color_scheme,
                    NativeColorSchemeEntryMutationTarget::Variable {
                        variable: &entry_mutation.variable,
                    },
                    entry_mutation.mutation_start,
                    entry_mutation.mutation_max_start,
                    overrides,
                )?;
            }
            Some(parsed)
        }
        NativeColorSchemeLuaSource::LoadScheme {
            path,
            variable,
            entry_mutation,
        } => {
            let mut parsed = apply_toml_color_scheme_file_overrides(Path::new(&path), overrides)?;
            if let Some(variable) = variable.as_ref() {
                parsed |= apply_lua_color_variable_mutation_overrides(config, variable, overrides)?;
            }
            if let Some(entry_mutation) = entry_mutation.as_ref() {
                parsed |= apply_lua_color_scheme_entry_mutation_overrides(
                    config,
                    color_scheme,
                    NativeColorSchemeEntryMutationTarget::Variable {
                        variable: &entry_mutation.variable,
                    },
                    entry_mutation.mutation_start,
                    entry_mutation.mutation_max_start,
                    overrides,
                )?;
            }
            Some(parsed)
        }
        NativeColorSchemeLuaSource::Builtin {
            name,
            variable,
            entry_mutation,
        } => {
            let mut parsed = apply_builtin_color_scheme_overrides(&name, overrides)?;
            if let Some(variable) = variable.as_ref() {
                parsed |= apply_lua_color_variable_mutation_overrides(config, variable, overrides)?;
            }
            if let Some(entry_mutation) = entry_mutation.as_ref() {
                parsed |= apply_lua_color_scheme_entry_mutation_overrides(
                    config,
                    color_scheme,
                    NativeColorSchemeEntryMutationTarget::Variable {
                        variable: &entry_mutation.variable,
                    },
                    entry_mutation.mutation_start,
                    entry_mutation.mutation_max_start,
                    overrides,
                )?;
            }
            Some(parsed)
        }
        NativeColorSchemeLuaSource::DefaultColors {
            variable,
            entry_mutation,
        } => {
            let mut parsed = apply_wezterm_default_colors_overrides(overrides);
            if let Some(variable) = variable.as_ref() {
                parsed |= apply_lua_color_variable_mutation_overrides(config, variable, overrides)?;
            }
            if let Some(entry_mutation) = entry_mutation.as_ref() {
                parsed |= apply_lua_color_scheme_entry_mutation_overrides(
                    config,
                    color_scheme,
                    NativeColorSchemeEntryMutationTarget::Variable {
                        variable: &entry_mutation.variable,
                    },
                    entry_mutation.mutation_start,
                    entry_mutation.mutation_max_start,
                    overrides,
                )?;
            }
            Some(parsed)
        }
    }
}

fn native_color_scheme_palette_from_lua_source(
    config: &str,
    color_scheme: &str,
    source: NativeColorSchemeLuaSource<'_>,
) -> Option<NativeResolvedPalette> {
    let mut overrides = NativeConfigSnapshot::default();
    apply_lua_color_scheme_source_overrides(config, color_scheme, source, &mut overrides)?;
    Some(native_resolved_palette_from_overrides(&overrides))
}

fn native_color_schemes_lua_table_from_query(
    config: &str,
    color_schemes: &str,
) -> Option<HashMap<String, NativeResolvedPalette>> {
    let table = color_schemes
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let mut schemes = HashMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((name, colors)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let name = split_lua_table_key_from_query(name.trim())?;
        let source = color_scheme_lua_source_value_from_query(config, colors, false)?;
        let palette = native_color_scheme_palette_from_lua_source(config, &name, source)?;
        schemes.insert(name, palette);
    }

    Some(schemes)
}

fn apply_lua_color_scheme_config_assignments_to_map(
    config: &str,
    receiver: &str,
    schemes: &mut HashMap<String, NativeResolvedPalette>,
) -> Option<bool> {
    let mut parsed = false;

    for start in lua_top_level_statement_start_indices_before_offset(config, config.len())? {
        let Some(rest) = lua_config_receiver_prefix_rest(config.get(start..)?, receiver) else {
            continue;
        };
        let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('.') else {
            continue;
        };
        let Some(rest) = rest.strip_prefix("color_schemes") else {
            continue;
        };
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            continue;
        }
        let Some((name, rest)) = color_scheme_lua_table_assignment_key_from_query(rest) else {
            continue;
        };
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let source = color_scheme_lua_source_value_from_query(config, value, true)?;
        let palette = native_color_scheme_palette_from_lua_source(config, &name, source)?;
        schemes.insert(name, palette);
        parsed = true;
    }

    Some(parsed)
}

fn apply_lua_color_scheme_config_mutations_to_map(
    config: &str,
    receiver: &str,
    schemes: &mut HashMap<String, NativeResolvedPalette>,
) -> Option<bool> {
    let mut parsed = false;
    let scheme_names = schemes.keys().cloned().collect::<Vec<_>>();

    for name in scheme_names {
        let mut overrides = NativeConfigSnapshot::default();
        if !apply_lua_color_scheme_entry_mutation_overrides(
            config,
            &name,
            NativeColorSchemeEntryMutationTarget::Config { receiver },
            0,
            config.len(),
            &mut overrides,
        )? {
            continue;
        }
        let base = schemes.remove(&name)?;
        schemes.insert(
            name,
            native_resolved_palette_with_overrides(&base, &overrides),
        );
        parsed = true;
    }

    Some(parsed)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn native_color_schemes_from_wezterm_lua_config(
    config: &str,
    receiver: &str,
) -> Option<Option<HashMap<String, NativeResolvedPalette>>> {
    let mut parsed = false;
    let mut schemes = HashMap::new();

    if let Some(color_schemes) =
        lua_config_table_or_static_variable_assignment_from_query(config, "color_schemes")
    {
        schemes = native_color_schemes_lua_table_from_query(config, color_schemes)?;
        parsed = true;
    }

    parsed |= apply_lua_color_scheme_config_assignments_to_map(config, receiver, &mut schemes)?;
    if parsed {
        apply_lua_color_scheme_config_mutations_to_map(config, receiver, &mut schemes)?;
    }

    Some(parsed.then_some(schemes))
}

fn color_scheme_lua_mutation_range_from_config_query(
    source: &str,
    color_scheme: &str,
    receiver: &str,
) -> Option<(usize, usize)> {
    let mut mutation_start = 0usize;

    if let Some(color_schemes) =
        lua_config_table_or_static_variable_assignment_from_query(source, "color_schemes")
        && color_scheme_lua_source_from_query(source, color_schemes, color_scheme)?.is_some()
    {
        mutation_start = lua_source_slice_end_offset(source, color_schemes)?;
    }

    if let Some(assignment_end) =
        color_scheme_lua_assignment_end_from_query(source, color_scheme, receiver)?
    {
        mutation_start = assignment_end;
    }

    Some((mutation_start, source.len()))
}

fn apply_lua_selected_color_scheme_mutation_overrides(
    source: &str,
    color_scheme: &str,
    receiver: &str,
    mutation_start: usize,
    mutation_max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    apply_lua_color_scheme_entry_mutation_overrides(
        source,
        color_scheme,
        NativeColorSchemeEntryMutationTarget::Config { receiver },
        mutation_start,
        mutation_max_start,
        overrides,
    )
}

fn apply_lua_color_scheme_entry_mutation_overrides(
    source: &str,
    color_scheme: &str,
    target: NativeColorSchemeEntryMutationTarget<'_>,
    mutation_start: usize,
    mutation_max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    let colors = lua_color_scheme_entry_mutation_table_from_query(
        source,
        color_scheme,
        target,
        mutation_start,
        mutation_max_start,
    )?;
    if let Some(colors) = colors {
        parsed |= apply_lua_colors_table_overrides(
            Some(LuaStaticSource {
                source,
                max_start: mutation_max_start,
            }),
            &colors,
            overrides,
        )?;
    }
    parsed |= apply_lua_color_scheme_entry_indexed_mutation_overrides(
        source,
        color_scheme,
        target,
        mutation_start,
        mutation_max_start,
        overrides,
    )?;
    parsed |= apply_lua_color_scheme_entry_palette_slot_mutation_overrides(
        source,
        color_scheme,
        target,
        mutation_start,
        mutation_max_start,
        overrides,
    )?;
    parsed |= apply_lua_color_scheme_entry_tab_bar_mutation_overrides(
        source,
        color_scheme,
        target,
        mutation_start,
        mutation_max_start,
        overrides,
    )?;
    parsed |= apply_lua_color_scheme_entry_color_spec_mutation_overrides(
        source,
        color_scheme,
        target,
        mutation_start,
        mutation_max_start,
        overrides,
    )?;

    Some(parsed)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_color_scheme_entry_mutation_table_from_query(
    source: &str,
    color_scheme: &str,
    target: NativeColorSchemeEntryMutationTarget<'_>,
    min_start: usize,
    max_start: usize,
) -> Option<Option<String>> {
    let mut fields = Vec::new();

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if start < min_start {
            continue;
        }
        let Some(rest) = lua_color_scheme_entry_mutation_rest_from_query(
            source.get(start..)?,
            color_scheme,
            target,
        ) else {
            continue;
        };
        let Some((field_name, rest)) = lua_color_variable_mutation_field_from_query(rest) else {
            continue;
        };
        let rest = lua_trim_start_comments(rest)?;
        if matches!(field_name.as_str(), "indexed" | "tab_bar") {
            continue;
        }
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        if value.is_empty() {
            continue;
        }
        fields.push(format!("{field_name} = {value}"));
    }

    Some((!fields.is_empty()).then(|| format!("{{\n{}\n}}", fields.join(",\n"))))
}

fn apply_lua_color_scheme_entry_indexed_mutation_overrides(
    source: &str,
    color_scheme: &str,
    target: NativeColorSchemeEntryMutationTarget<'_>,
    min_start: usize,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if start < min_start {
            continue;
        }
        let Some(rest) = lua_color_scheme_entry_mutation_rest_from_query(
            source.get(start..)?,
            color_scheme,
            target,
        ) else {
            continue;
        };
        let Some((field_name, rest)) = lua_color_variable_mutation_field_from_query(rest) else {
            continue;
        };
        if field_name != "indexed" {
            continue;
        }

        let rest = lua_trim_start_comments(rest)?;
        if let Some((index, rest)) = lua_color_variable_mutation_array_index_from_query(rest) {
            if !(16..=255).contains(&index) {
                return None;
            }
            let rest = lua_trim_start_comments(rest)?;
            let Some(value) = rest.strip_prefix('=') else {
                continue;
            };
            let value = lua_color_variable_mutation_value_literal_from_query(value)?;
            let value = parse_maybe_quoted_query_text(value)?;
            let mut palette = overrides.indexed_palette.unwrap_or([None; 256]);
            palette[index] = Some(lua_opaque_color_from_query_with_static_source(
                Some(LuaStaticSource {
                    source,
                    max_start: start,
                }),
                &value,
            )?);
            overrides.indexed_palette = Some(palette);
            parsed = true;
            continue;
        }

        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        parsed |= apply_lua_colors_table_overrides(
            Some(LuaStaticSource {
                source,
                max_start: start,
            }),
            &format!("{{\nindexed = {value}\n}}"),
            overrides,
        )?;
    }

    Some(parsed)
}

fn apply_lua_color_scheme_entry_palette_slot_mutation_overrides(
    source: &str,
    color_scheme: &str,
    target: NativeColorSchemeEntryMutationTarget<'_>,
    min_start: usize,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if start < min_start {
            continue;
        }
        let Some(rest) = lua_color_scheme_entry_mutation_rest_from_query(
            source.get(start..)?,
            color_scheme,
            target,
        ) else {
            continue;
        };
        let Some((field_name, rest)) = lua_color_variable_mutation_field_from_query(rest) else {
            continue;
        };
        let Some(offset) = (match field_name.as_str() {
            "ansi" => Some(0),
            "brights" => Some(8),
            _ => None,
        }) else {
            continue;
        };
        let rest = lua_trim_start_comments(rest)?;
        let mut palette = overrides
            .ansi_palette
            .unwrap_or(DEFAULT_ANSI_PALETTE_COLORS);

        if let Some((index, rest)) = lua_color_variable_mutation_array_index_from_query(rest) {
            if !(1..=8).contains(&index) {
                return None;
            }
            let rest = lua_trim_start_comments(rest)?;
            let Some(value) = rest.strip_prefix('=') else {
                continue;
            };
            let value = lua_color_variable_mutation_value_literal_from_query(value)?;
            let value = parse_maybe_quoted_query_text(value)?;
            palette[offset + index - 1] = lua_opaque_color_from_query_with_static_source(
                Some(LuaStaticSource {
                    source,
                    max_start: start,
                }),
                &value,
            )?;
        } else {
            let Some(value) = rest.strip_prefix('=') else {
                continue;
            };
            let value = lua_color_variable_mutation_value_literal_from_query(value)?;
            let trimmed_value = value.trim();
            if trimmed_value.starts_with('{')
                && trimmed_value
                    .strip_prefix('{')?
                    .strip_suffix('}')?
                    .trim()
                    .is_empty()
            {
                continue;
            }
            let values = split_lua_table_string_array_with_static_source(
                Some(LuaStaticSource {
                    source,
                    max_start: start,
                }),
                value,
            )?;
            let colors = values
                .iter()
                .map(|value| {
                    lua_opaque_color_from_query_with_static_source(
                        Some(LuaStaticSource {
                            source,
                            max_start: start,
                        }),
                        value,
                    )
                })
                .collect::<Option<Vec<_>>>()?;
            let colors = <[Color; 8]>::try_from(colors).ok()?;
            palette[offset..offset + 8].copy_from_slice(&colors);
        }

        overrides.ansi_palette = Some(palette);
        parsed = true;
    }

    Some(parsed)
}

fn apply_lua_color_scheme_entry_tab_bar_mutation_overrides(
    source: &str,
    color_scheme: &str,
    target: NativeColorSchemeEntryMutationTarget<'_>,
    min_start: usize,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if start < min_start {
            continue;
        }
        let Some(rest) = lua_color_scheme_entry_mutation_rest_from_query(
            source.get(start..)?,
            color_scheme,
            target,
        ) else {
            continue;
        };
        let Some((field_name, rest)) = lua_color_variable_mutation_field_from_query(rest) else {
            continue;
        };
        if field_name != "tab_bar" {
            continue;
        }

        if apply_lua_tab_bar_color_mutation_rest(source, rest, start, overrides)? {
            parsed = true;
        }
    }

    Some(parsed)
}

fn apply_lua_color_scheme_entry_color_spec_mutation_overrides(
    source: &str,
    color_scheme: &str,
    target: NativeColorSchemeEntryMutationTarget<'_>,
    min_start: usize,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if start < min_start {
            continue;
        }
        let Some(rest) = lua_color_scheme_entry_mutation_rest_from_query(
            source.get(start..)?,
            color_scheme,
            target,
        ) else {
            continue;
        };
        let Some((field_name, rest)) =
            lua_color_variable_mutation_field_from_query_with_static_key(source, rest, start)
        else {
            continue;
        };
        if !lua_color_spec_field_name(&field_name) {
            continue;
        }
        let Some((variant_name, rest)) =
            lua_color_variable_mutation_field_from_query_with_static_key(source, rest, start)
        else {
            continue;
        };
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        let color = lua_color_spec_from_query_with_static_source(
            Some(LuaStaticSource {
                source,
                max_start: start,
            }),
            &format!("{{ {variant_name} = {value} }}"),
        )?;

        if apply_lua_color_spec_field_override(overrides, &field_name, color) {
            parsed = true;
        }
    }

    Some(parsed)
}

fn lua_color_scheme_entry_mutation_rest_from_query<'a>(
    query: &'a str,
    color_scheme: &str,
    target: NativeColorSchemeEntryMutationTarget<'_>,
) -> Option<&'a str> {
    let rest = match target {
        NativeColorSchemeEntryMutationTarget::Config { receiver } => {
            let rest = lua_config_receiver_prefix_rest(query, receiver)?;
            let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
            let rest = rest.strip_prefix("color_schemes")?;
            if rest.chars().next().is_some_and(is_lua_identifier_character) {
                return None;
            }
            rest
        }
        NativeColorSchemeEntryMutationTarget::Variable { variable } => {
            let rest = query.strip_prefix(variable)?;
            if rest.chars().next().is_some_and(is_lua_identifier_character) {
                return None;
            }
            rest
        }
    };
    let (name, rest) = color_scheme_lua_table_assignment_key_from_query(rest)?;
    if name != color_scheme {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    if rest.starts_with('=') {
        return None;
    }

    Some(rest)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_scheme_lua_source_from_config_query<'a>(
    source: &'a str,
    color_scheme: &str,
    receiver: &str,
) -> Option<Option<NativeColorSchemeLuaSource<'a>>> {
    let mut selected = None;

    if let Some(color_schemes) =
        lua_config_table_or_static_variable_assignment_from_query(source, "color_schemes")
    {
        selected = color_scheme_lua_source_from_query(source, color_schemes, color_scheme)?;
    }

    if let Some(source) =
        color_scheme_lua_variable_assignment_from_config_query(source, color_scheme)?
    {
        selected = Some(source);
    }

    if let Some(source) = color_scheme_lua_assignment_from_query(source, color_scheme, receiver)? {
        selected = Some(source);
    }

    Some(selected)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_scheme_lua_variable_assignment_from_config_query<'a>(
    source: &'a str,
    color_scheme: &str,
) -> Option<Option<NativeColorSchemeLuaSource<'a>>> {
    let Some(variable) = lua_config_assignment_from_query(
        source,
        "color_schemes",
        lua_identifier_literal_from_query,
    ) else {
        return Some(None);
    };
    let Some(max_start) = lua_source_slice_start_offset(source, variable) else {
        return Some(None);
    };
    color_scheme_lua_variable_assignment_before_offset(source, variable, color_scheme, max_start)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_scheme_lua_variable_assignment_before_offset<'a>(
    source: &'a str,
    variable: &str,
    color_scheme: &str,
    max_start: usize,
) -> Option<Option<NativeColorSchemeLuaSource<'a>>> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let Some(rest) = source.get(start..)?.strip_prefix(variable) else {
            continue;
        };
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            continue;
        }
        let Some((name, rest)) = color_scheme_lua_table_assignment_key_from_query(rest) else {
            continue;
        };
        if name != color_scheme {
            continue;
        }
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let mutation_start = color_scheme_lua_source_value_end_from_query(source, value, true)?;
        let entry_mutation = NativeColorSchemeEntryVariableReference {
            variable: variable.to_owned(),
            mutation_start,
            mutation_max_start: max_start,
        };
        selected = Some(
            color_scheme_lua_source_value_from_query(source, value, true)?
                .with_entry_mutation(entry_mutation),
        );
    }

    Some(selected)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_scheme_lua_source_from_query<'a>(
    source: &'a str,
    color_schemes: &'a str,
    color_scheme: &str,
) -> Option<Option<NativeColorSchemeLuaSource<'a>>> {
    let table = color_schemes
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let mut selected = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((name, colors)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let name = split_lua_table_key_from_query(name.trim())?;
        if name != color_scheme {
            continue;
        }
        selected = Some(color_scheme_lua_source_value_from_query(
            source, colors, false,
        )?);
    }

    Some(selected)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_scheme_lua_assignment_from_query<'a>(
    source: &'a str,
    color_scheme: &str,
    receiver: &str,
) -> Option<Option<NativeColorSchemeLuaSource<'a>>> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        let Some(rest) = lua_config_receiver_prefix_rest(source.get(start..)?, receiver) else {
            continue;
        };
        let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('.') else {
            continue;
        };
        let Some(rest) = rest.strip_prefix("color_schemes") else {
            continue;
        };
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            continue;
        }
        let Some((name, rest)) = color_scheme_lua_table_assignment_key_from_query(rest) else {
            continue;
        };
        if name != color_scheme {
            continue;
        }
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        selected = Some(color_scheme_lua_source_value_from_query(
            source, value, true,
        )?);
    }

    Some(selected)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_scheme_lua_assignment_end_from_query(
    source: &str,
    color_scheme: &str,
    receiver: &str,
) -> Option<Option<usize>> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        let Some(rest) = lua_config_receiver_prefix_rest(source.get(start..)?, receiver) else {
            continue;
        };
        let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('.') else {
            continue;
        };
        let Some(rest) = rest.strip_prefix("color_schemes") else {
            continue;
        };
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            continue;
        }
        let Some((name, rest)) = color_scheme_lua_table_assignment_key_from_query(rest) else {
            continue;
        };
        if name != color_scheme {
            continue;
        }
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        selected = Some(color_scheme_lua_source_value_end_from_query(
            source, value, true,
        )?);
    }

    Some(selected)
}

fn color_scheme_lua_table_assignment_key_from_query(query: &str) -> Option<(String, &str)> {
    let query = lua_trim_start_comments(query)?;
    if let Some(rest) = query.strip_prefix('.') {
        let field_name = lua_identifier_literal_from_query(rest)?;
        return Some((field_name.to_owned(), rest.get(field_name.len()..)?));
    }

    let rest = query.strip_prefix('[')?;
    let rest = lua_trim_start_comments(rest)?;
    let literal = lua_quoted_string_literal_from_query(rest)
        .or_else(|| lua_long_bracket_literal_from_query(rest))?;
    let name = parse_maybe_quoted_query_text(literal)?;
    let rest = lua_trim_start_comments(rest.get(literal.len()..)?)?;
    let rest = rest.strip_prefix(']')?;

    Some((name, rest))
}

fn color_scheme_lua_source_value_from_query<'a>(
    source: &'a str,
    colors: &'a str,
    allow_trailing_lines: bool,
) -> Option<NativeColorSchemeLuaSource<'a>> {
    let colors = lua_trim_start_comments(colors)?.trim();
    if colors.starts_with('{') {
        let colors = lua_braced_table_literal_from_query(colors)?;
        colors.strip_prefix('{')?.strip_suffix('}')?;
        return Some(NativeColorSchemeLuaSource::Table {
            colors,
            variable: None,
            entry_mutation: None,
        });
    }

    let value_query = if allow_trailing_lines {
        colors
            .split_once('\n')
            .map_or(colors, |(colors, _)| colors)
            .trim()
    } else {
        colors
    };
    if let Some(path) =
        lua_wezterm_color_load_scheme_path_from_query_with_static_source(source, value_query)
    {
        return Some(NativeColorSchemeLuaSource::LoadScheme {
            path,
            variable: None,
            entry_mutation: None,
        });
    }
    if let Some(name) =
        lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(source, value_query)
    {
        return Some(NativeColorSchemeLuaSource::Builtin {
            name,
            variable: None,
            entry_mutation: None,
        });
    }
    if let Some(name) = lua_whole_map_builtin_color_scheme_name_from_query(source, value_query) {
        return Some(NativeColorSchemeLuaSource::Builtin {
            name,
            variable: None,
            entry_mutation: None,
        });
    }
    if lua_wezterm_default_colors_from_query_with_static_source(source, value_query).is_some() {
        return Some(NativeColorSchemeLuaSource::DefaultColors {
            variable: None,
            entry_mutation: None,
        });
    }

    let variable_query = value_query;
    let variable = lua_identifier_literal_from_query(variable_query)?;
    if !variable_query[variable.len()..].trim().is_empty() {
        return None;
    }
    let max_start = lua_source_slice_start_offset(source, value_query)?;
    lua_color_variable_source_before_offset(source, variable, max_start)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_color_variable_source_before_offset<'a>(
    source: &'a str,
    variable: &str,
    reference_start: usize,
) -> Option<NativeColorSchemeLuaSource<'a>> {
    let statements = lua_top_level_logical_statements_before_offset(source, reference_start)?;
    let reference_statement_index = statements
        .iter()
        .rposition(|statement| statement.start <= reference_start)?;
    let reference_statement_start = statements.get(reference_statement_index)?.start;
    let mut selected = None;
    let mut selected_cell = None;
    let mut current_cell = 0usize;
    let mut next_cell = 1usize;
    let mut function_bindings = BTreeMap::<String, usize>::new();
    let mut closure_capture_cells = Vec::<Option<usize>>::new();
    let mut mutation_events = Vec::new();

    for statement in statements.iter().take(reference_statement_index) {
        let start = statement.start;
        let end = statement.end.min(reference_statement_start);
        let statement = source.get(start..end)?;

        if lua_statement_declares_local_identifier(statement, variable)? {
            current_cell = next_cell;
            next_cell = next_cell.saturating_add(1);
            selected = None;
            selected_cell = None;
            mutation_events.clear();
        }

        if let Some(function) = lua_static_builtin_scheme_function_definition_name(statement)? {
            let closure_id = closure_capture_cells.len();
            let captured_cell =
                lua_static_query_contains_identifier(statement, variable)?.then_some(current_cell);
            closure_capture_cells.push(captured_cell);
            function_bindings.insert(function, closure_id);
            continue;
        }

        if let Some((alias, value)) = lua_single_identifier_assignment_from_query(statement) {
            let value = lua_trim_start_comments(value)?;
            let target = lua_identifier_literal_from_query(value).and_then(|target| {
                lua_static_builtin_scheme_tail_is_statement_end(value.get(target.len()..)?)?
                    .then_some(target)
            });
            if alias != variable
                && let Some(closure_id) =
                    target.and_then(|target| function_bindings.get(target).copied())
            {
                function_bindings.insert(alias, closure_id);
                continue;
            }
            if alias != variable {
                function_bindings.remove(&alias);
            }
        }

        let called_or_escaped_capture = selected_cell.is_some_and(|selected_cell| {
            function_bindings.iter().any(|(name, closure_id)| {
                closure_capture_cells.get(*closure_id).copied().flatten() == Some(selected_cell)
                    && lua_static_query_contains_identifier(statement, name).unwrap_or(true)
            })
        });
        if called_or_escaped_capture {
            selected = None;
            selected_cell = None;
            mutation_events.clear();
            continue;
        }

        if let Some(binding) =
            lua_color_variable_known_binding_from_query(source, statement, variable)?
        {
            selected = Some(binding);
            selected_cell = Some(current_cell);
            mutation_events.clear();
            continue;
        }

        if lua_color_variable_whole_assignment_value_from_query(statement, variable).is_some() {
            selected = None;
            selected_cell = None;
            mutation_events.clear();
            continue;
        }

        if selected.is_none() {
            continue;
        }
        if !lua_static_query_contains_identifier(statement, variable)? {
            continue;
        }
        if let Some(event) = lua_palette_mutation_event_from_statement(
            source,
            LuaLogicalStatement { start, end },
            variable,
        )? {
            mutation_events.push(event);
            continue;
        }
        selected = None;
        selected_cell = None;
        mutation_events.clear();
    }

    selected.map(|(binding, _mutation_min_start)| {
        let variable = Some(NativeLoadSchemeVariableReference {
            name: variable.to_owned(),
            mutation_max_start: reference_start,
            mutation_events,
        });
        match binding {
            NativeColorSchemeLuaSource::Table { colors, .. } => NativeColorSchemeLuaSource::Table {
                colors,
                variable,
                entry_mutation: None,
            },
            NativeColorSchemeLuaSource::LoadScheme { path, .. } => {
                NativeColorSchemeLuaSource::LoadScheme {
                    path,
                    variable,
                    entry_mutation: None,
                }
            }
            NativeColorSchemeLuaSource::Builtin { name, .. } => {
                NativeColorSchemeLuaSource::Builtin {
                    name,
                    variable,
                    entry_mutation: None,
                }
            }
            NativeColorSchemeLuaSource::DefaultColors { .. } => {
                NativeColorSchemeLuaSource::DefaultColors {
                    variable,
                    entry_mutation: None,
                }
            }
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LuaLogicalStatement {
    start: usize,
    end: usize,
}

fn lua_top_level_logical_statements_before_offset(
    source: &str,
    max_start: usize,
) -> Option<Vec<LuaLogicalStatement>> {
    let starts = lua_top_level_statement_start_indices_before_offset(source, max_start)?;
    let mut statements = Vec::new();
    let mut statement_index = 0usize;

    while statement_index < starts.len() {
        let start = *starts.get(statement_index)?;
        let mut next_statement_index = statement_index + 1;
        while let Some(next_start) = starts.get(next_statement_index).copied() {
            let next_end = starts
                .get(next_statement_index + 1)
                .copied()
                .unwrap_or(max_start);
            if !lua_static_load_scheme_path_statements_continue_across_boundary(
                source.get(start..next_start)?,
                source.get(next_start..next_end)?,
            )? {
                break;
            }
            next_statement_index += 1;
        }

        let end = starts
            .get(next_statement_index)
            .copied()
            .unwrap_or(max_start);
        statements.push(LuaLogicalStatement { start, end });
        statement_index = next_statement_index;
    }

    Some(statements)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_color_variable_known_binding_from_query<'a>(
    source: &'a str,
    statement: &'a str,
    variable: &str,
) -> Option<Option<(NativeColorSchemeLuaSource<'a>, usize)>> {
    let value = lua_color_variable_whole_assignment_value_from_query(statement, variable);
    let Some(value) = value else {
        return Some(None);
    };

    if let Some(table) = lua_braced_table_literal_from_query(value)
        && lua_static_builtin_scheme_tail_is_statement_end(value.get(table.len()..)?)?
    {
        let binding_end = lua_source_slice_end_offset(source, table)?;
        return Some(Some((
            NativeColorSchemeLuaSource::Table {
                colors: table,
                variable: None,
                entry_mutation: None,
            },
            binding_end,
        )));
    }
    if let Some(path) = lua_load_scheme_assignment_path_from_query(source, statement, variable) {
        let binding_end = lua_source_slice_end_offset(source, value.trim_end())?;
        return Some(Some((
            NativeColorSchemeLuaSource::LoadScheme {
                path,
                variable: None,
                entry_mutation: None,
            },
            binding_end,
        )));
    }
    if let Some(name) = lua_builtin_color_scheme_assignment_from_query(source, statement, variable)
    {
        let binding_end = lua_source_slice_end_offset(source, value.trim_end())?;
        return Some(Some((
            NativeColorSchemeLuaSource::Builtin {
                name,
                variable: None,
                entry_mutation: None,
            },
            binding_end,
        )));
    }
    if lua_wezterm_default_colors_from_query_with_static_source(source, value).is_some() {
        let binding_end = lua_source_slice_end_offset(source, value.trim_end())?;
        return Some(Some((
            NativeColorSchemeLuaSource::DefaultColors {
                variable: None,
                entry_mutation: None,
            },
            binding_end,
        )));
    }

    Some(None)
}

fn lua_color_variable_whole_assignment_value_from_query<'a>(
    statement: &'a str,
    variable: &str,
) -> Option<&'a str> {
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let statement = lua_trim_start_comments(statement)?;
    let statement = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let (targets, value) = split_lua_static_load_scheme_path_assignment_statement(statement)?;
    let targets = split_lua_top_level_arguments(targets)?;
    let target = targets.first()?;
    if lua_static_load_scheme_path_assignment_target_identifier(target).as_deref() != Some(variable)
    {
        return None;
    }
    lua_trim_start_comments(value)
}

fn lua_statement_declares_local_identifier(statement: &str, variable: &str) -> Option<bool> {
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let statement = lua_trim_start_comments(statement)?;
    if !lua_source_keyword_at(statement, 0, "local") {
        return Some(false);
    }
    let declaration = lua_trim_start_comments(statement.get("local".len()..)?)?;
    if lua_source_keyword_at(declaration, 0, "function") {
        let declaration = lua_trim_start_comments(declaration.get("function".len()..)?)?;
        return Some(lua_identifier_literal_from_query(declaration) == Some(variable));
    }

    let targets = split_lua_static_load_scheme_path_assignment_statement(declaration)
        .map_or(declaration, |(targets, _)| targets);
    let targets = split_lua_top_level_arguments(targets)?;
    Some(targets.iter().any(|target| {
        lua_static_load_scheme_path_assignment_target_identifier(target).as_deref()
            == Some(variable)
    }))
}

fn lua_single_identifier_assignment_from_query(statement: &str) -> Option<(String, &str)> {
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let statement = lua_trim_start_comments(statement)?;
    let statement = if lua_source_keyword_at(statement, 0, "local") {
        let statement = lua_trim_start_comments(statement.get("local".len()..)?)?;
        if lua_source_keyword_at(statement, 0, "function") {
            return None;
        }
        statement
    } else {
        statement
    };
    let (targets, value) = split_lua_static_load_scheme_path_assignment_statement(statement)?;
    let targets = split_lua_top_level_arguments(targets)?;
    let [target] = targets.as_slice() else {
        return None;
    };
    Some((
        lua_static_load_scheme_path_assignment_target_identifier(target)?,
        value,
    ))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_palette_mutation_event_from_statement(
    source: &str,
    statement_range: LuaLogicalStatement,
    variable: &str,
) -> Option<Option<LuaPaletteMutationEvent>> {
    let statement = source.get(statement_range.start..statement_range.end)?;
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let statement = lua_trim_start_comments(statement)?;
    let Some((targets, value)) = split_lua_static_load_scheme_path_assignment_statement(statement)
    else {
        return Some(None);
    };
    let targets = split_lua_top_level_arguments(targets)?;
    let [target] = targets.as_slice() else {
        return Some(None);
    };
    let normalized = lua_static_load_scheme_path_query_without_comments(target)?;
    let target = normalized.trim();
    let Some(rest) = target.strip_prefix(variable) else {
        return Some(None);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(None);
    }
    let rest = rest.trim_start();
    if !rest.starts_with('.') && !rest.starts_with('[') {
        return Some(None);
    }
    let Some((field_name, _)) = lua_color_variable_mutation_field_from_query_with_static_key(
        source,
        rest,
        statement_range.start,
    ) else {
        return Some(None);
    };
    if !lua_palette_mutation_field_name(&field_name) {
        return Some(Some(LuaPaletteMutationEvent {
            statement: statement_range,
        }));
    }
    if lua_static_query_contains_identifier(value, variable)?
        || !lua_color_variable_mutation_rhs_is_exact_static_expression(value)?
    {
        return Some(None);
    }

    let mut probe = NativeConfigSnapshot::default();
    let parsed = apply_lua_color_variable_mutation_statement_overrides(
        source,
        variable,
        statement_range.start,
        statement_range.end,
        &mut probe,
    )?;
    let supported_empty_initialization =
        lua_color_variable_statement_is_supported_empty_table_initialization(
            source,
            target,
            value,
            statement_range.start,
            variable,
        )?;
    Some(
        (parsed || supported_empty_initialization).then_some(LuaPaletteMutationEvent {
            statement: statement_range,
        }),
    )
}

fn lua_palette_mutation_field_name(field_name: &str) -> bool {
    lua_color_spec_field_name(field_name)
        || matches!(
            field_name,
            "foreground"
                | "background"
                | "ansi"
                | "brights"
                | "indexed"
                | "selection_fg"
                | "selection_bg"
                | "cursor_bg"
                | "cursor_border"
                | "cursor_fg"
                | "compose_cursor"
                | "split"
                | "scrollbar_thumb"
                | "tab_bar"
                | "visual_bell"
        )
}

fn lua_color_variable_mutation_rhs_is_exact_static_expression(value: &str) -> Option<bool> {
    let value = lua_trim_start_comments(value)?;
    let literal = lua_braced_table_literal_from_query(value)
        .or_else(|| lua_quoted_string_literal_from_query(value))
        .or_else(|| lua_long_bracket_literal_from_query(value))
        .or_else(|| lua_bool_literal_from_query(value))
        .or_else(|| lua_signed_number_literal_from_query(value));
    let Some(literal) = literal else {
        return Some(true);
    };
    lua_static_builtin_scheme_tail_is_statement_end(value.get(literal.len()..)?)
}

fn lua_color_variable_statement_is_supported_empty_table_initialization(
    source: &str,
    target: &str,
    value: &str,
    statement_start: usize,
    variable: &str,
) -> Option<bool> {
    let value = lua_trim_start_comments(value)?;
    let Some(table) = lua_braced_table_literal_from_query(value) else {
        return Some(false);
    };
    if !table
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim()
        .is_empty()
        || !lua_static_builtin_scheme_tail_is_statement_end(value.get(table.len()..)?)?
    {
        return Some(false);
    }

    let target = lua_trim_start_comments(target)?;
    let Some(rest) = target.strip_prefix(variable) else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let Some((field_name, rest)) =
        lua_color_variable_mutation_field_from_query_with_static_key(source, rest, statement_start)
    else {
        return Some(false);
    };
    if lua_color_spec_field_name(&field_name)
        || matches!(field_name.as_str(), "ansi" | "brights" | "indexed")
    {
        return Some(lua_trim_start_comments(rest)?.is_empty());
    }
    if field_name != "tab_bar" {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?;
    if rest.is_empty() {
        return Some(true);
    }
    let Some((item_name, rest)) =
        lua_color_variable_mutation_field_from_query_with_static_key(source, rest, statement_start)
    else {
        return Some(false);
    };
    Some(lua_tab_bar_item_color_name(&item_name) && lua_trim_start_comments(rest)?.is_empty())
}

fn color_scheme_lua_source_value_end_from_query(
    source: &str,
    colors: &str,
    allow_trailing_lines: bool,
) -> Option<usize> {
    let colors = lua_trim_start_comments(colors)?.trim_start();
    if colors.starts_with('{') {
        let colors = lua_braced_table_literal_from_query(colors)?;
        return lua_source_slice_end_offset(source, colors);
    }

    let value_query = if allow_trailing_lines {
        colors.split_once('\n').map_or(colors, |(colors, _)| colors)
    } else {
        colors
    };
    color_scheme_lua_source_value_from_query(source, colors, allow_trailing_lines)?;
    lua_source_slice_end_offset(source, value_query)
}

fn lua_source_slice_end_offset(source: &str, slice: &str) -> Option<usize> {
    let start = lua_source_slice_start_offset(source, slice)?;
    start
        .checked_add(slice.len())
        .filter(|end| *end <= source.len())
}

fn lua_source_slice_start_offset(source: &str, slice: &str) -> Option<usize> {
    let source_start = source.as_ptr() as usize;
    let slice_start = slice.as_ptr() as usize;
    let start = slice_start.checked_sub(source_start)?;
    start
        .checked_add(slice.len())
        .filter(|end| *end <= source.len())?;
    Some(start)
}

fn lua_static_table_variable_assignment_table_from_query<'a>(
    query: &'a str,
    variable: &str,
) -> Option<&'a str> {
    let after_variable = query.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let rest = lua_trim_start_comments(after_variable)?;
    let value = rest.strip_prefix('=')?;
    lua_braced_table_literal_from_query(lua_trim_start_comments(value)?)
}

fn apply_toml_color_scheme_dirs_overrides(
    color_scheme_dirs: &[String],
    color_scheme: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    for color_scheme_dir in color_scheme_dirs {
        if let Some(parsed) = apply_toml_color_scheme_dir_overrides(
            Path::new(color_scheme_dir),
            color_scheme,
            overrides,
        )? {
            return Some(parsed);
        }
    }

    Some(false)
}

fn apply_default_toml_color_scheme_dirs_overrides(
    color_scheme: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    for color_scheme_dir in default_toml_color_scheme_dirs() {
        if let Some(parsed) =
            apply_toml_color_scheme_dir_overrides(&color_scheme_dir, color_scheme, overrides)?
        {
            return Some(parsed);
        }
    }

    Some(false)
}

fn apply_builtin_color_scheme_overrides(
    color_scheme: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let Some(scheme) = builtin_color_scheme_toml(color_scheme) else {
        return Some(false);
    };
    let scheme = toml::from_str::<toml::Value>(scheme).ok()?;
    let colors = scheme.as_table().and_then(|table| table.get("colors"))?;
    apply_toml_colors_table_overrides(colors, overrides)
}

fn builtin_color_scheme_toml(color_scheme: &str) -> Option<&'static str> {
    rterm_config::schemes::get(color_scheme)
}

fn default_toml_color_scheme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(windows)]
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("colors");
        dirs.push(exe_path);
    }

    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(
            PathBuf::from(home)
                .join(".config")
                .join("wezterm")
                .join("colors"),
        );
    }

    dirs
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn apply_toml_color_scheme_dir_overrides(
    color_scheme_dir: &Path,
    color_scheme: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<Option<bool>> {
    let Ok(entries) = fs::read_dir(color_scheme_dir) else {
        return Some(None);
    };
    let mut paths = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
    paths.sort();

    for path in paths {
        if !toml_color_scheme_file_path_candidate(&path) {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(scheme) = toml::from_str::<toml::Value>(&contents) else {
            continue;
        };
        if !toml_color_scheme_name_matches(&scheme, &path, color_scheme) {
            continue;
        }
        let Some(colors) = scheme.as_table().and_then(|table| table.get("colors")) else {
            continue;
        };
        return apply_toml_colors_table_overrides(colors, overrides).map(Some);
    }

    Some(None)
}

fn apply_toml_color_scheme_file_overrides(
    path: &Path,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let contents = fs::read_to_string(path).ok()?;
    let scheme = toml::from_str::<toml::Value>(&contents).ok()?;
    let colors = scheme.as_table().and_then(|table| table.get("colors"))?;
    apply_toml_colors_table_overrides(colors, overrides)
}

fn toml_color_scheme_file_path_candidate(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(extension) if extension.eq_ignore_ascii_case("toml")
    )
}

fn toml_color_scheme_name_matches(scheme: &toml::Value, path: &Path, color_scheme: &str) -> bool {
    let metadata_name = scheme
        .as_table()
        .and_then(|table| table.get("metadata"))
        .and_then(|metadata| metadata.as_table())
        .and_then(|metadata| metadata.get("name"))
        .and_then(|name| name.as_str());
    let file_stem = path.file_stem().and_then(|file_stem| file_stem.to_str());
    matches!(
        metadata_name.or(file_stem),
        Some(scheme_name) if scheme_name == color_scheme
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn apply_toml_colors_table_overrides(
    colors: &toml::Value,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    if let Some(foreground_color) = toml_color_table_field_from_query(colors, "foreground")? {
        overrides.foreground_color = Some(foreground_color);
        parsed = true;
    }
    if let Some(background_color) = toml_color_table_field_from_query(colors, "background")? {
        overrides.background_color = Some(background_color);
        parsed = true;
    }
    if let Some(ansi_colors) = toml_color_array_table_field_from_query(colors, "ansi")? {
        let mut palette = overrides
            .ansi_palette
            .unwrap_or(DEFAULT_ANSI_PALETTE_COLORS);
        palette[..8].copy_from_slice(&ansi_colors);
        overrides.ansi_palette = Some(palette);
        parsed = true;
    }
    if let Some(bright_colors) = toml_color_array_table_field_from_query(colors, "brights")? {
        let mut palette = overrides
            .ansi_palette
            .unwrap_or(DEFAULT_ANSI_PALETTE_COLORS);
        palette[8..].copy_from_slice(&bright_colors);
        overrides.ansi_palette = Some(palette);
        parsed = true;
    }
    if let Some(indexed_palette) = toml_indexed_palette_table_field_from_query(colors)? {
        overrides.indexed_palette = Some(indexed_palette);
        parsed = true;
    }
    if let Some(selection_fg_color) = toml_selection_fg_table_field_from_query(colors)? {
        overrides.selection_fg_color = Some(selection_fg_color);
        parsed = true;
    }
    if let Some(selection_bg_color) = toml_selection_bg_table_field_from_query(colors)? {
        overrides.selection_bg_color = Some(selection_bg_color);
        parsed = true;
    }
    if let Some(cursor_bg_color) = toml_color_table_field_from_query(colors, "cursor_bg")? {
        overrides.cursor_bg_color = Some(cursor_bg_color);
        parsed = true;
    }
    if let Some(cursor_border_color) = toml_color_table_field_from_query(colors, "cursor_border")? {
        overrides.cursor_border_color = Some(cursor_border_color);
        parsed = true;
    }
    if let Some(cursor_fg_color) = toml_color_table_field_from_query(colors, "cursor_fg")? {
        overrides.cursor_fg_color = Some(cursor_fg_color);
        parsed = true;
    }
    if let Some(compose_cursor_color) = toml_color_table_field_from_query(colors, "compose_cursor")?
    {
        overrides.compose_cursor_color = Some(compose_cursor_color);
        parsed = true;
    }
    if let Some(split_color) = toml_color_table_field_from_query(colors, "split")? {
        overrides.split_color = Some(split_color);
        parsed = true;
    }
    if let Some(scrollbar_thumb_color) =
        toml_color_table_field_from_query(colors, "scrollbar_thumb")?
    {
        overrides.scrollbar_thumb_color = Some(scrollbar_thumb_color);
        parsed = true;
    }
    if let Some(tab_bar_background_color) = toml_tab_bar_background_from_query(colors)? {
        overrides.tab_bar_background_color = Some(tab_bar_background_color);
        parsed = true;
    }
    if let Some(tab_bar_inactive_tab_edge_color) =
        toml_tab_bar_inactive_tab_edge_from_query(colors)?
    {
        overrides.tab_bar_inactive_tab_edge_color = Some(tab_bar_inactive_tab_edge_color);
        parsed = true;
    }
    if let Some(active_tab_colors) = toml_tab_bar_item_colors_from_query(colors, "active_tab")? {
        overrides.tab_bar_active_tab_colors = active_tab_colors;
        parsed = true;
    }
    if let Some(inactive_tab_colors) = toml_tab_bar_item_colors_from_query(colors, "inactive_tab")?
    {
        overrides.tab_bar_inactive_tab_colors = inactive_tab_colors;
        parsed = true;
    }
    if let Some(inactive_tab_hover_colors) =
        toml_tab_bar_item_colors_from_query(colors, "inactive_tab_hover")?
    {
        overrides.tab_bar_inactive_tab_hover_colors = inactive_tab_hover_colors;
        parsed = true;
    }
    if let Some(new_tab_colors) = toml_tab_bar_item_colors_from_query(colors, "new_tab")? {
        overrides.tab_bar_new_tab_colors = new_tab_colors;
        parsed = true;
    }
    if let Some(new_tab_hover_colors) =
        toml_tab_bar_item_colors_from_query(colors, "new_tab_hover")?
    {
        overrides.tab_bar_new_tab_hover_colors = new_tab_hover_colors;
        parsed = true;
    }
    if let Some(visual_bell_color) = toml_color_table_field_from_query(colors, "visual_bell")? {
        overrides.visual_bell_color = Some(visual_bell_color);
        parsed = true;
    }
    if let Some(color) =
        toml_color_spec_table_field_from_query(colors, "copy_mode_active_highlight_bg")?
    {
        overrides.copy_mode_active_highlight_bg = Some(color);
        parsed = true;
    }
    if let Some(color) =
        toml_color_spec_table_field_from_query(colors, "copy_mode_active_highlight_fg")?
    {
        overrides.copy_mode_active_highlight_fg = Some(color);
        parsed = true;
    }
    if let Some(color) =
        toml_color_spec_table_field_from_query(colors, "copy_mode_inactive_highlight_bg")?
    {
        overrides.copy_mode_inactive_highlight_bg = Some(color);
        parsed = true;
    }
    if let Some(color) =
        toml_color_spec_table_field_from_query(colors, "copy_mode_inactive_highlight_fg")?
    {
        overrides.copy_mode_inactive_highlight_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "quick_select_label_bg")? {
        overrides.quick_select_label_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "quick_select_label_fg")? {
        overrides.quick_select_label_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "quick_select_match_bg")? {
        overrides.quick_select_match_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "quick_select_match_fg")? {
        overrides.quick_select_match_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "input_selector_label_bg")?
    {
        overrides.input_selector_label_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "input_selector_label_fg")?
    {
        overrides.input_selector_label_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "launcher_label_bg")? {
        overrides.launcher_label_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_spec_table_field_from_query(colors, "launcher_label_fg")? {
        overrides.launcher_label_fg = Some(color);
        parsed = true;
    }

    Some(parsed)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_table_field<'a>(
    value: &'a toml::Value,
    field_name: &str,
) -> Option<Option<&'a toml::Value>> {
    let table = value.as_table()?;
    Some(table.get(field_name))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_string_table_field_from_query<'a>(
    value: &'a toml::Value,
    field_name: &str,
) -> Option<Option<&'a str>> {
    let Some(value) = toml_table_field(value, field_name)? else {
        return Some(None);
    };
    Some(Some(value.as_str()?))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_color_table_field_from_query(
    value: &toml::Value,
    field_name: &str,
) -> Option<Option<Color>> {
    let Some(value) = toml_string_table_field_from_query(value, field_name)? else {
        return Some(None);
    };
    Some(Some(lua_opaque_color_from_query(value)?))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_selection_fg_table_field_from_query(value: &toml::Value) -> Option<Option<Option<Color>>> {
    let Some(value) = toml_string_table_field_from_query(value, "selection_fg")? else {
        return Some(None);
    };
    Some(Some(if value.eq_ignore_ascii_case("none") {
        None
    } else {
        lua_selection_foreground_color_from_query(value)?
    }))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_selection_bg_table_field_from_query(value: &toml::Value) -> Option<Option<Color>> {
    let Some(value) = toml_string_table_field_from_query(value, "selection_bg")? else {
        return Some(None);
    };
    Some(Some(lua_color_from_query(value)?))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_color_array_table_field_from_query(
    value: &toml::Value,
    field_name: &str,
) -> Option<Option<[Color; 8]>> {
    let Some(value) = toml_table_field(value, field_name)? else {
        return Some(None);
    };
    let parsed = value
        .as_array()?
        .iter()
        .map(|value| value.as_str().and_then(lua_opaque_color_from_query))
        .collect::<Option<Vec<_>>>()?;
    Some(Some(<[Color; 8]>::try_from(parsed).ok()?))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_indexed_palette_table_field_from_query(
    value: &toml::Value,
) -> Option<Option<[Option<Color>; 256]>> {
    let Some(indexed) = toml_table_field(value, "indexed")? else {
        return Some(None);
    };
    let mut palette = [None; 256];
    for (index, color) in indexed.as_table()? {
        let index = index.parse::<usize>().ok()?;
        if !(16..=255).contains(&index) || palette[index].is_some() {
            return None;
        }
        palette[index] = Some(lua_opaque_color_from_query(color.as_str()?)?);
    }
    Some(Some(palette))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_tab_bar_background_from_query(value: &toml::Value) -> Option<Option<Color>> {
    let Some(tab_bar) = toml_table_field(value, "tab_bar")? else {
        return Some(None);
    };
    toml_color_table_field_from_query(tab_bar, "background")
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_tab_bar_inactive_tab_edge_from_query(value: &toml::Value) -> Option<Option<Color>> {
    let Some(tab_bar) = toml_table_field(value, "tab_bar")? else {
        return Some(None);
    };
    toml_color_table_field_from_query(tab_bar, "inactive_tab_edge")
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_tab_bar_item_colors_from_query(
    value: &toml::Value,
    item_name: &str,
) -> Option<Option<NativeTabBarItemColors>> {
    let Some(tab_bar) = toml_table_field(value, "tab_bar")? else {
        return Some(None);
    };
    let Some(item) = toml_table_field(tab_bar, item_name)? else {
        return Some(None);
    };

    let mut colors = NativeTabBarItemColors::default();
    let mut parsed = false;
    if let Some(color) = toml_color_table_field_from_query(item, "fg_color")? {
        colors.fg_color = Some(color);
        parsed = true;
    }
    if let Some(color) = toml_color_table_field_from_query(item, "bg_color")? {
        colors.bg_color = Some(color);
        parsed = true;
    }
    if let Some(intensity) = toml_string_table_field_from_query(item, "intensity")? {
        colors.intensity = Some(tab_bar_item_intensity_from_query(intensity)?);
        parsed = true;
    }
    if let Some(underline) = toml_string_table_field_from_query(item, "underline")? {
        colors.underline = Some(tab_bar_item_underline_from_query(underline)?);
        parsed = true;
    }
    if let Some(italic) = toml_bool_table_field_from_query(item, "italic")? {
        colors.italic = Some(italic);
        parsed = true;
    }
    if let Some(strikethrough) = toml_bool_table_field_from_query(item, "strikethrough")? {
        colors.strikethrough = Some(strikethrough);
        parsed = true;
    }

    Some(parsed.then_some(colors))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_bool_table_field_from_query(value: &toml::Value, field_name: &str) -> Option<Option<bool>> {
    let Some(value) = toml_table_field(value, field_name)? else {
        return Some(None);
    };
    Some(Some(value.as_bool()?))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn toml_color_spec_table_field_from_query(
    value: &toml::Value,
    field_name: &str,
) -> Option<Option<NativeColorSpec>> {
    let Some(value) = toml_table_field(value, field_name)? else {
        return Some(None);
    };
    Some(Some(toml_color_spec_from_query(value)?))
}

fn toml_color_spec_from_query(value: &toml::Value) -> Option<NativeColorSpec> {
    let table = value.as_table()?;
    let mut color = None;
    for (key, value) in table {
        if color.is_some() {
            return None;
        }
        let value = value.as_str()?;
        color = Some(match key.as_str() {
            "Color" => NativeColorSpec::Color(lua_opaque_color_from_query(value)?),
            "AnsiColor" => NativeColorSpec::AnsiColor(NativeAnsiColor::parse(value)?),
            _ => return None,
        });
    }
    color
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn apply_lua_colors_table_overrides(
    static_source: Option<LuaStaticSource<'_>>,
    colors: &str,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    if let Some(foreground_color) =
        color_lua_table_field_from_query_with_static_source(static_source, colors, "foreground")?
    {
        overrides.foreground_color = Some(foreground_color);
        parsed = true;
    }
    if let Some(background_color) =
        color_lua_table_field_from_query_with_static_source(static_source, colors, "background")?
    {
        overrides.background_color = Some(background_color);
        parsed = true;
    }
    if let Some(ansi_colors) =
        color_array_lua_table_field_from_query_with_static_source(static_source, colors, "ansi")?
    {
        let mut palette = overrides
            .ansi_palette
            .unwrap_or(DEFAULT_ANSI_PALETTE_COLORS);
        palette[..8].copy_from_slice(&ansi_colors);
        overrides.ansi_palette = Some(palette);
        parsed = true;
    }
    if let Some(bright_colors) =
        color_array_lua_table_field_from_query_with_static_source(static_source, colors, "brights")?
    {
        let mut palette = overrides
            .ansi_palette
            .unwrap_or(DEFAULT_ANSI_PALETTE_COLORS);
        palette[8..].copy_from_slice(&bright_colors);
        overrides.ansi_palette = Some(palette);
        parsed = true;
    }
    if let Some(indexed_palette) =
        indexed_palette_lua_table_field_from_query_with_static_source(static_source, colors)?
    {
        overrides.indexed_palette = Some(indexed_palette);
        parsed = true;
    }
    if let Some(selection_fg_color) =
        selection_fg_lua_table_field_from_query_with_static_source(static_source, colors)?
    {
        overrides.selection_fg_color = Some(selection_fg_color);
        parsed = true;
    }
    if let Some(selection_bg_color) =
        selection_bg_lua_table_field_from_query_with_static_source(static_source, colors)?
    {
        overrides.selection_bg_color = Some(selection_bg_color);
        parsed = true;
    }
    if let Some(cursor_bg_color) =
        color_lua_table_field_from_query_with_static_source(static_source, colors, "cursor_bg")?
    {
        overrides.cursor_bg_color = Some(cursor_bg_color);
        parsed = true;
    }
    if let Some(cursor_border_color) =
        color_lua_table_field_from_query_with_static_source(static_source, colors, "cursor_border")?
    {
        overrides.cursor_border_color = Some(cursor_border_color);
        parsed = true;
    }
    if let Some(cursor_fg_color) =
        color_lua_table_field_from_query_with_static_source(static_source, colors, "cursor_fg")?
    {
        overrides.cursor_fg_color = Some(cursor_fg_color);
        parsed = true;
    }
    if let Some(compose_cursor_color) = color_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "compose_cursor",
    )? {
        overrides.compose_cursor_color = Some(compose_cursor_color);
        parsed = true;
    }
    if let Some(split_color) =
        color_lua_table_field_from_query_with_static_source(static_source, colors, "split")?
    {
        overrides.split_color = Some(split_color);
        parsed = true;
    }
    if let Some(scrollbar_thumb_color) = color_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "scrollbar_thumb",
    )? {
        overrides.scrollbar_thumb_color = Some(scrollbar_thumb_color);
        parsed = true;
    }
    if let Some(tab_bar_background_color) =
        tab_bar_background_lua_table_from_query(static_source, colors)?
    {
        overrides.tab_bar_background_color = Some(tab_bar_background_color);
        parsed = true;
    }
    if let Some(tab_bar_inactive_tab_edge_color) =
        tab_bar_inactive_tab_edge_lua_table_from_query(static_source, colors)?
    {
        overrides.tab_bar_inactive_tab_edge_color = Some(tab_bar_inactive_tab_edge_color);
        parsed = true;
    }
    if let Some(active_tab_colors) =
        tab_bar_item_colors_lua_table_from_query(static_source, colors, "active_tab")?
    {
        overrides.tab_bar_active_tab_colors = active_tab_colors;
        parsed = true;
    }
    if let Some(inactive_tab_colors) =
        tab_bar_item_colors_lua_table_from_query(static_source, colors, "inactive_tab")?
    {
        overrides.tab_bar_inactive_tab_colors = inactive_tab_colors;
        parsed = true;
    }
    if let Some(inactive_tab_hover_colors) =
        tab_bar_item_colors_lua_table_from_query(static_source, colors, "inactive_tab_hover")?
    {
        overrides.tab_bar_inactive_tab_hover_colors = inactive_tab_hover_colors;
        parsed = true;
    }
    if let Some(new_tab_colors) =
        tab_bar_item_colors_lua_table_from_query(static_source, colors, "new_tab")?
    {
        overrides.tab_bar_new_tab_colors = new_tab_colors;
        parsed = true;
    }
    if let Some(new_tab_hover_colors) =
        tab_bar_item_colors_lua_table_from_query(static_source, colors, "new_tab_hover")?
    {
        overrides.tab_bar_new_tab_hover_colors = new_tab_hover_colors;
        parsed = true;
    }
    if let Some(visual_bell_color) = visual_bell_color_lua_table_from_query(static_source, colors)?
    {
        overrides.visual_bell_color = Some(visual_bell_color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "copy_mode_active_highlight_bg",
    )? {
        overrides.copy_mode_active_highlight_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "copy_mode_active_highlight_fg",
    )? {
        overrides.copy_mode_active_highlight_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "copy_mode_inactive_highlight_bg",
    )? {
        overrides.copy_mode_inactive_highlight_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "copy_mode_inactive_highlight_fg",
    )? {
        overrides.copy_mode_inactive_highlight_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "quick_select_label_bg",
    )? {
        overrides.quick_select_label_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "quick_select_label_fg",
    )? {
        overrides.quick_select_label_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "quick_select_match_bg",
    )? {
        overrides.quick_select_match_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "quick_select_match_fg",
    )? {
        overrides.quick_select_match_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "input_selector_label_bg",
    )? {
        overrides.input_selector_label_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "input_selector_label_fg",
    )? {
        overrides.input_selector_label_fg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "launcher_label_bg",
    )? {
        overrides.launcher_label_bg = Some(color);
        parsed = true;
    }
    if let Some(color) = color_spec_lua_table_field_from_query_with_static_source(
        static_source,
        colors,
        "launcher_label_fg",
    )? {
        overrides.launcher_label_fg = Some(color);
        parsed = true;
    }

    Some(parsed)
}

fn apply_lua_color_variable_mutation_overrides(
    source: &str,
    variable: &NativeLoadSchemeVariableReference,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;
    let mut unfinished_composites = HashSet::new();
    for event in &variable.mutation_events {
        if let Some((field_name, empty)) = lua_color_variable_mutation_field_state(
            source,
            &variable.name,
            event.statement.start,
            event.statement.end.min(variable.mutation_max_start),
        )? {
            if empty
                && (lua_color_spec_field_name(&field_name)
                    || matches!(field_name.as_str(), "ansi" | "brights"))
            {
                unfinished_composites.insert(field_name);
            } else {
                unfinished_composites.remove(&field_name);
            }
        }
        parsed |= apply_lua_color_variable_mutation_statement_overrides(
            source,
            &variable.name,
            event.statement.start,
            event.statement.end.min(variable.mutation_max_start),
            overrides,
        )?;
    }

    if !unfinished_composites.is_empty() {
        return None;
    }

    Some(parsed)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_color_variable_mutation_field_state(
    source: &str,
    variable: &str,
    statement_start: usize,
    statement_end: usize,
) -> Option<Option<(String, bool)>> {
    let statement = source.get(statement_start..statement_end)?;
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let statement = lua_trim_start_comments(statement)?;
    let Some((target, value)) = split_lua_static_load_scheme_path_assignment_statement(statement)
    else {
        return Some(None);
    };
    let target = lua_static_load_scheme_path_query_without_comments(target)?;
    let target = target.trim();
    let Some(rest) = target.strip_prefix(variable) else {
        return Some(None);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(None);
    }
    let Some((field_name, _)) =
        lua_color_variable_mutation_field_from_query_with_static_key(source, rest, statement_start)
    else {
        return Some(None);
    };
    let value = lua_trim_start_comments(value)?;
    let empty = lua_braced_table_literal_from_query(value).is_some_and(|table| {
        table
            .strip_prefix('{')
            .and_then(|table| table.strip_suffix('}'))
            .is_some_and(|table| table.trim().is_empty())
    });
    Some(Some((field_name, empty)))
}

fn apply_lua_color_variable_mutation_statement_overrides(
    source: &str,
    variable: &str,
    statement_start: usize,
    statement_end: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = apply_lua_color_variable_empty_table_replacement(
        source,
        variable,
        statement_start,
        statement_end,
        overrides,
    )?;
    if parsed {
        return Some(true);
    }
    if let Some(colors) = lua_color_variable_mutation_table_from_query(
        source,
        variable,
        statement_start,
        statement_end,
    ) {
        parsed |= apply_lua_colors_table_overrides(
            Some(LuaStaticSource {
                source,
                max_start: statement_start,
            }),
            &colors,
            overrides,
        )?;
    }
    parsed |= apply_lua_color_variable_palette_slot_mutation_overrides(
        source,
        variable,
        statement_start,
        statement_end,
        overrides,
    )?;
    parsed |= apply_lua_color_variable_indexed_palette_slot_mutation_overrides(
        source,
        variable,
        statement_start,
        statement_end,
        overrides,
    )?;
    parsed |= apply_lua_color_variable_tab_bar_mutation_overrides(
        source,
        variable,
        statement_start,
        statement_end,
        overrides,
    )?;
    parsed |= apply_lua_color_variable_color_spec_mutation_overrides(
        source,
        variable,
        statement_start,
        statement_end,
        overrides,
    )?;

    Some(parsed)
}

fn apply_lua_color_variable_empty_table_replacement(
    source: &str,
    variable: &str,
    statement_start: usize,
    statement_end: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let statement = source.get(statement_start..statement_end)?;
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let statement = lua_trim_start_comments(statement)?;
    let Some((target, value)) = split_lua_static_load_scheme_path_assignment_statement(statement)
    else {
        return Some(false);
    };
    let value = lua_trim_start_comments(value)?;
    let Some(table) = lua_braced_table_literal_from_query(value) else {
        return Some(false);
    };
    if !table
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim()
        .is_empty()
        || !lua_static_builtin_scheme_tail_is_statement_end(value.get(table.len()..)?)?
    {
        return Some(false);
    }
    let target = lua_static_load_scheme_path_query_without_comments(target)?;
    let target = target.trim();
    let rest = target.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let (field_name, rest) = lua_color_variable_mutation_field_from_query_with_static_key(
        source,
        rest,
        statement_start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    if rest.is_empty() && matches!(field_name.as_str(), "ansi" | "brights") {
        return Some(true);
    }
    if rest.is_empty() && clear_lua_color_spec_field_override(overrides, &field_name) {
        return Some(true);
    }
    if field_name == "indexed" && rest.is_empty() {
        overrides.indexed_palette = Some([None; 256]);
        return Some(true);
    }
    if field_name != "tab_bar" {
        return Some(false);
    }
    if rest.is_empty() {
        overrides.tab_bar_background_color = None;
        overrides.tab_bar_inactive_tab_edge_color = None;
        overrides.tab_bar_active_tab_colors = NativeTabBarItemColors::default();
        overrides.tab_bar_inactive_tab_colors = NativeTabBarItemColors::default();
        overrides.tab_bar_inactive_tab_hover_colors = NativeTabBarItemColors::default();
        overrides.tab_bar_new_tab_colors = NativeTabBarItemColors::default();
        overrides.tab_bar_new_tab_hover_colors = NativeTabBarItemColors::default();
        return Some(true);
    }
    let (item_name, rest) = lua_color_variable_mutation_field_from_query_with_static_key(
        source,
        rest,
        statement_start,
    )?;
    if !lua_trim_start_comments(rest)?.is_empty() {
        return Some(false);
    }
    let colors = match item_name.as_str() {
        "active_tab" => &mut overrides.tab_bar_active_tab_colors,
        "inactive_tab" => &mut overrides.tab_bar_inactive_tab_colors,
        "inactive_tab_hover" => &mut overrides.tab_bar_inactive_tab_hover_colors,
        "new_tab" => &mut overrides.tab_bar_new_tab_colors,
        "new_tab_hover" => &mut overrides.tab_bar_new_tab_hover_colors,
        _ => return Some(false),
    };
    *colors = NativeTabBarItemColors::default();
    Some(true)
}

fn clear_lua_color_spec_field_override(
    overrides: &mut NativeConfigSnapshot,
    field_name: &str,
) -> bool {
    match field_name {
        "copy_mode_active_highlight_bg" => overrides.copy_mode_active_highlight_bg = None,
        "copy_mode_active_highlight_fg" => overrides.copy_mode_active_highlight_fg = None,
        "copy_mode_inactive_highlight_bg" => overrides.copy_mode_inactive_highlight_bg = None,
        "copy_mode_inactive_highlight_fg" => overrides.copy_mode_inactive_highlight_fg = None,
        "quick_select_label_bg" => overrides.quick_select_label_bg = None,
        "quick_select_label_fg" => overrides.quick_select_label_fg = None,
        "quick_select_match_bg" => overrides.quick_select_match_bg = None,
        "quick_select_match_fg" => overrides.quick_select_match_fg = None,
        "input_selector_label_bg" => overrides.input_selector_label_bg = None,
        "input_selector_label_fg" => overrides.input_selector_label_fg = None,
        "launcher_label_bg" => overrides.launcher_label_bg = None,
        "launcher_label_fg" => overrides.launcher_label_fg = None,
        _ => return false,
    }
    true
}

fn apply_lua_config_colors_tab_bar_mutation_overrides(
    source: &str,
    receiver: &str,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let Some(after_receiver) = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)
        else {
            continue;
        };
        let after_receiver = lua_trim_start_comments(after_receiver)?;
        let Some(rest) = lua_config_field_access_rest_from_query_with_static_key(
            source,
            after_receiver,
            "colors",
            start,
        ) else {
            continue;
        };
        let Some((field_name, rest)) =
            lua_color_variable_mutation_field_from_query_with_static_key(source, rest, start)
        else {
            continue;
        };
        if field_name != "tab_bar" {
            continue;
        }

        if apply_lua_tab_bar_color_mutation_rest(source, rest, start, overrides)? {
            parsed = true;
        }
    }

    Some(parsed)
}

fn apply_lua_color_variable_color_spec_mutation_overrides(
    source: &str,
    variable: &str,
    statement_start: usize,
    statement_end: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let statement = source.get(statement_start..statement_end)?;
    let Some(rest) = statement.strip_prefix(variable) else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let rest = statement.get(variable.len()..)?.trim_start();
    let Some((field_name, rest)) =
        lua_color_variable_mutation_field_from_query_with_static_key(source, rest, statement_start)
    else {
        return Some(false);
    };
    if !lua_color_spec_field_name(&field_name) {
        return Some(false);
    }
    let Some((variant_name, rest)) =
        lua_color_variable_mutation_field_from_query_with_static_key(source, rest, statement_start)
    else {
        return Some(false);
    };
    let rest = lua_trim_start_comments(rest)?;
    let Some(value) = rest.strip_prefix('=') else {
        return Some(false);
    };
    let value = lua_color_variable_mutation_value_literal_from_query(value)?;
    let color = lua_color_spec_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: statement_start,
        }),
        &format!("{{ {variant_name} = {value} }}"),
    )?;

    Some(apply_lua_color_spec_field_override(
        overrides,
        &field_name,
        color,
    ))
}

fn apply_lua_config_colors_color_spec_mutation_overrides(
    source: &str,
    receiver: &str,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let mut parsed = false;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let Some(after_receiver) = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)
        else {
            continue;
        };
        let after_receiver = lua_trim_start_comments(after_receiver)?;
        let Some(rest) = lua_config_field_access_rest_from_query_with_static_key(
            source,
            after_receiver,
            "colors",
            start,
        ) else {
            continue;
        };
        let Some((field_name, rest)) =
            lua_color_variable_mutation_field_from_query_with_static_key(source, rest, start)
        else {
            continue;
        };
        if !lua_color_spec_field_name(&field_name) {
            continue;
        }
        let Some((variant_name, rest)) =
            lua_color_variable_mutation_field_from_query_with_static_key(source, rest, start)
        else {
            continue;
        };
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        let color = lua_color_spec_from_query_with_static_source(
            Some(LuaStaticSource {
                source,
                max_start: start,
            }),
            &format!("{{ {variant_name} = {value} }}"),
        )?;

        if apply_lua_color_spec_field_override(overrides, &field_name, color) {
            parsed = true;
        }
    }

    Some(parsed)
}

fn lua_color_spec_field_name(field_name: &str) -> bool {
    matches!(
        field_name,
        "copy_mode_active_highlight_bg"
            | "copy_mode_active_highlight_fg"
            | "copy_mode_inactive_highlight_bg"
            | "copy_mode_inactive_highlight_fg"
            | "quick_select_label_bg"
            | "quick_select_label_fg"
            | "quick_select_match_bg"
            | "quick_select_match_fg"
            | "input_selector_label_bg"
            | "input_selector_label_fg"
            | "launcher_label_bg"
            | "launcher_label_fg"
    )
}

fn apply_lua_color_spec_field_override(
    overrides: &mut NativeConfigSnapshot,
    field_name: &str,
    color: NativeColorSpec,
) -> bool {
    match field_name {
        "copy_mode_active_highlight_bg" => overrides.copy_mode_active_highlight_bg = Some(color),
        "copy_mode_active_highlight_fg" => overrides.copy_mode_active_highlight_fg = Some(color),
        "copy_mode_inactive_highlight_bg" => {
            overrides.copy_mode_inactive_highlight_bg = Some(color);
        }
        "copy_mode_inactive_highlight_fg" => {
            overrides.copy_mode_inactive_highlight_fg = Some(color);
        }
        "quick_select_label_bg" => overrides.quick_select_label_bg = Some(color),
        "quick_select_label_fg" => overrides.quick_select_label_fg = Some(color),
        "quick_select_match_bg" => overrides.quick_select_match_bg = Some(color),
        "quick_select_match_fg" => overrides.quick_select_match_fg = Some(color),
        "input_selector_label_bg" => overrides.input_selector_label_bg = Some(color),
        "input_selector_label_fg" => overrides.input_selector_label_fg = Some(color),
        "launcher_label_bg" => overrides.launcher_label_bg = Some(color),
        "launcher_label_fg" => overrides.launcher_label_fg = Some(color),
        _ => return false,
    }

    true
}

#[allow(dead_code)]
fn lua_config_string_assignment_from_query(source: &str, field: &str) -> Option<String> {
    lua_config_assignment_from_query(source, field, |value| {
        lua_static_string_assignment_value_from_query(source, value)
    })
    .and_then(parse_maybe_quoted_query_text)
}

fn lua_config_color_assignment_from_query(source: &str, field: &str) -> Option<Color> {
    let value =
        lua_config_assignment_from_query(source, field, lua_top_level_statement_value_from_query)?;
    let max_start = lua_source_slice_start_offset(source, value)?;
    let mut value_max_start = max_start;

    let value = if let Some(value) =
        lua_static_string_assignment_value_before_offset_from_query(source, value, max_start)
    {
        value_max_start = lua_source_slice_start_offset(source, value).unwrap_or(value_max_start);
        value
    } else {
        value
    };

    let value = parse_maybe_quoted_query_text(value)?;
    lua_color_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: value_max_start,
        }),
        &value,
    )
}

fn lua_config_opaque_color_assignment_from_query(source: &str, field: &str) -> Option<Color> {
    Some(opaque_color(lua_config_color_assignment_from_query(
        source, field,
    )?))
}

fn lua_config_integrated_title_button_color_assignment_from_query(
    source: &str,
) -> Option<NativeIntegratedTitleButtonColor> {
    lua_config_string_assignment_from_query(source, "integrated_title_button_color")
        .and_then(|value| NativeIntegratedTitleButtonColor::parse(&value))
        .or_else(|| {
            lua_config_opaque_color_assignment_from_query(source, "integrated_title_button_color")
                .map(NativeIntegratedTitleButtonColor::Color)
        })
}

#[allow(dead_code)]
fn lua_config_font_assignment_from_query(source: &str, field: &str) -> Option<NativeFontConfig> {
    lua_config_assignment_from_query(source, field, |value| {
        lua_static_string_assignment_value_from_query(source, value)
            .or_else(|| {
                lua_wezterm_font_call_assignment_value_from_query_with_static_source(source, value)
            })
            .or_else(|| {
                let max_start = lua_source_slice_start_offset(source, value)?;
                lua_static_wezterm_font_value_assignment_before_offset_from_query(
                    source, value, max_start,
                )
            })
    })
    .and_then(|value| parse_wezterm_font_config_value(source, value))
}

fn lua_config_font_rules_assignment_from_query(source: &str) -> Option<Vec<NativeFontRule>> {
    lua_config_table_assignment_with_insert_appends_with_max_start_from_query(source, "font_rules")
        .and_then(|rules| {
            native_font_rules_lua_table_from_query(
                Some(LuaStaticSource {
                    source,
                    max_start: rules.max_start,
                }),
                source,
                &rules.value,
            )
        })
        .or_else(|| {
            lua_config_table_or_static_variable_assignment_from_query(source, "font_rules")
                .and_then(|rules| {
                    let static_source = lua_source_slice_start_offset(source, rules)
                        .map(|max_start| LuaStaticSource { source, max_start });
                    native_font_rules_lua_table_from_query(static_source, source, rules)
                })
        })
}

fn lua_wezterm_font_call_assignment_value_from_query(query: &str) -> Option<&str> {
    lua_wezterm_font_with_fallback_call_assignment_value_from_query(query)
        .or_else(|| lua_wezterm_font_family_call_assignment_value_from_query(query))
}

fn lua_wezterm_font_call_assignment_value_from_query_with_static_source<'a>(
    source: &str,
    query: &'a str,
) -> Option<&'a str> {
    lua_wezterm_font_call_assignment_value_from_query(query)
        .or_else(|| lua_static_wezterm_font_call_assignment_value_from_query(source, query))
        .or_else(|| lua_static_wezterm_font_alias_call_assignment_value_from_query(source, query))
}

fn lua_wezterm_font_family_call_assignment_value_from_query(query: &str) -> Option<&str> {
    let query = lua_trim_start_comments(query)?;
    let mut rest = lua_function_name_rest_from_query(query, "wezterm.font")?;
    let mut rest_start = query.len() - rest.len();
    let parenthesized = rest.starts_with('(');
    if parenthesized {
        let stripped = rest.get('('.len_utf8()..)?;
        rest = stripped.trim_start();
        rest_start = query.len() - rest.len();
    }
    if let Some(table) = lua_braced_table_literal_from_query(rest) {
        let table_end = rest_start + table.len();
        if !parenthesized {
            return query.get(..table_end);
        }
        let after_table = lua_trim_start_comments(query.get(table_end..)?)?;
        if !after_table.starts_with(')') {
            return None;
        }
        return query.get(..query.len() - after_table.len() + ')'.len_utf8());
    }
    let quote = rest.find(['\'', '"'])?;
    let literal = lua_quoted_string_literal_from_query(rest.get(quote..)?)?;
    let literal_end = rest_start + quote + literal.len();
    let mut end = literal_end;
    let mut after_literal = lua_trim_start_comments(query.get(literal_end..)?)?;
    if let Some(after_comma) = after_literal.strip_prefix(',') {
        let attributes = lua_trim_start_comments(after_comma)?;
        if let Some(table) = lua_braced_table_literal_from_query(attributes) {
            end = query.len() - attributes.len() + table.len();
            after_literal = lua_trim_start_comments(query.get(end..)?)?;
        }
    }
    if after_literal.starts_with(')') {
        end = query.len() - after_literal.len() + ')'.len_utf8();
    }
    query.get(..end)
}

fn lua_wezterm_font_with_fallback_call_assignment_value_from_query(query: &str) -> Option<&str> {
    let query = lua_trim_start_comments(query)?;
    let mut rest = lua_function_name_rest_from_query(query, "wezterm.font_with_fallback")?;
    let mut rest_start = query.len() - rest.len();
    let parenthesized = rest.starts_with('(');
    if parenthesized {
        rest = rest.get('('.len_utf8()..)?.trim_start();
        rest_start = query.len() - rest.len();
    }
    let table = lua_braced_table_literal_from_query(rest)?;
    let table_end = rest_start + table.len();
    if !parenthesized {
        return query.get(..table_end);
    }
    let mut after_table = lua_trim_start_comments(query.get(table_end..)?)?;
    if let Some(after_comma) = after_table.strip_prefix(',') {
        let attributes = lua_trim_start_comments(after_comma)?;
        let table = lua_braced_table_literal_from_query(attributes)?;
        let end = query.len() - attributes.len() + table.len();
        after_table = lua_trim_start_comments(query.get(end..)?)?;
    }
    if !after_table.starts_with(')') {
        return None;
    }
    query.get(..query.len() - after_table.len() + ')'.len_utf8())
}

#[derive(Clone, Copy)]
enum LuaStaticWeztermFontAliasKind {
    Font,
    FontWithFallback,
}

impl LuaStaticWeztermFontAliasKind {
    fn normalized_prefix(self) -> &'static str {
        match self {
            Self::Font => "wezterm.font",
            Self::FontWithFallback => "wezterm.font_with_fallback",
        }
    }
}

fn lua_static_wezterm_font_alias_call_assignment_value_from_query<'a>(
    source: &str,
    query: &'a str,
) -> Option<&'a str> {
    let query = lua_trim_start_comments(query)?;
    let max_start = lua_source_slice_start_offset(source, query)?;
    let alias = lua_identifier_literal_from_query(query)?;
    let kind = lua_static_wezterm_font_alias_kind_before_offset(source, alias, max_start)??;
    let raw_rest = query.get(alias.len()..)?;
    let rest = lua_trim_start_comments(raw_rest)?;
    if !matches!(rest.chars().next()?, '(' | '\'' | '"' | '{') {
        return None;
    }

    let normalized_prefix = kind.normalized_prefix();
    let normalized = format!("{normalized_prefix}{rest}");
    let parsed = lua_wezterm_font_call_assignment_value_from_query(&normalized)?;
    let consumed_rest_len = parsed.len().checked_sub(normalized_prefix.len())?;
    let skipped_rest_len = raw_rest.len().checked_sub(rest.len())?;
    query.get(..alias.len() + skipped_rest_len + consumed_rest_len)
}

fn lua_static_wezterm_font_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = lua_trim_start_comments(query)?;
    let alias = lua_identifier_literal_from_query(query)?;
    let kind = lua_static_wezterm_font_alias_kind_before_offset(source, alias, max_start)??;
    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    if !matches!(rest.chars().next()?, '(' | '\'' | '"' | '{') {
        return None;
    }

    Some(format!("{}{}", kind.normalized_prefix(), rest))
}

fn lua_static_wezterm_font_call_assignment_value_from_query<'a>(
    source: &str,
    query: &'a str,
) -> Option<&'a str> {
    let query = lua_trim_start_comments(query)?;
    let max_start = lua_source_slice_start_offset(source, query)?;
    let (kind, raw_rest, rest) =
        lua_static_wezterm_font_call_kind_and_rest_from_query(source, query, max_start)?;
    let skipped_rest_len = raw_rest.len().checked_sub(rest.len())?;
    let prefix_len = query.len().checked_sub(raw_rest.len())? + skipped_rest_len;
    let normalized_prefix = kind.normalized_prefix();
    let normalized = format!("{normalized_prefix}{rest}");
    let parsed = lua_wezterm_font_call_assignment_value_from_query(&normalized)?;
    let consumed_rest_len = parsed.len().checked_sub(normalized_prefix.len())?;
    query.get(..prefix_len + consumed_rest_len)
}

fn lua_static_wezterm_font_call_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = lua_trim_start_comments(query)?;
    let (kind, _, rest) =
        lua_static_wezterm_font_call_kind_and_rest_from_query(source, query, max_start)?;
    Some(format!("{}{}", kind.normalized_prefix(), rest))
}

fn lua_static_wezterm_font_call_kind_and_rest_from_query<'a>(
    source: &str,
    query: &'a str,
    max_start: usize,
) -> Option<(LuaStaticWeztermFontAliasKind, &'a str, &'a str)> {
    let rest = lua_static_wezterm_receiver_rest_from_query(source, max_start, query)?;
    let rest = lua_trim_start_comments(rest)?;
    let static_source = Some(LuaStaticSource { source, max_start });
    let (field, raw_rest) =
        lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
    let kind = match field.as_str() {
        "font" => LuaStaticWeztermFontAliasKind::Font,
        "font_with_fallback" => LuaStaticWeztermFontAliasKind::FontWithFallback,
        _ => return None,
    };
    let rest = lua_trim_start_comments(raw_rest)?;
    if !matches!(rest.chars().next()?, '(' | '\'' | '"' | '{') {
        return None;
    }
    Some((kind, raw_rest, rest))
}

fn lua_static_wezterm_font_value_assignment_before_offset_from_query<'a>(
    source: &'a str,
    query: &str,
    max_start: usize,
) -> Option<&'a str> {
    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        let Some(rest) = rest.strip_prefix(variable) else {
            continue;
        };
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            continue;
        }
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        selected = lua_top_level_statement_value_from_query(value).and_then(|value| {
            lua_wezterm_font_call_assignment_value_from_query_with_static_source(source, value)
        });
    }

    selected
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_static_wezterm_font_alias_kind_before_offset(
    source: &str,
    alias: &str,
    max_start: usize,
) -> Option<Option<LuaStaticWeztermFontAliasKind>> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        let Some(rest) = rest.strip_prefix(alias) else {
            continue;
        };
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            continue;
        }
        let rest = lua_trim_start_comments(rest)?;
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        selected = lua_static_wezterm_font_alias_kind_from_value_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_font_alias_kind_from_value_query(
    source: &str,
    max_start: usize,
    value: &str,
) -> Option<LuaStaticWeztermFontAliasKind> {
    let value = lua_trim_start_comments(value)?;
    let rest = if let Some(rest) = lua_static_wezterm_require_receiver_rest_from_query(value) {
        rest
    } else {
        lua_static_wezterm_receiver_rest_from_query(source, max_start, value)?
    };
    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource { source, max_start }),
        rest,
    )?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    match field.as_str() {
        "font" => Some(LuaStaticWeztermFontAliasKind::Font),
        "font_with_fallback" => Some(LuaStaticWeztermFontAliasKind::FontWithFallback),
        _ => None,
    }
}

#[allow(dead_code)]
fn lua_config_bool_assignment_from_query(source: &str, field: &str) -> Option<bool> {
    lua_config_assignment_from_query(source, field, |value| {
        lua_static_bool_assignment_value_from_query(source, value)
    })
    .and_then(|value| value.parse().ok())
}

#[allow(dead_code)]
fn lua_config_usize_assignment_from_query(source: &str, field: &str) -> Option<usize> {
    lua_config_assignment_from_query(source, field, |value| {
        lua_static_number_assignment_value_from_query(
            source,
            value,
            lua_unsigned_integer_literal_from_query,
        )
    })
    .and_then(|value| value.parse().ok())
}

#[allow(dead_code)]
fn lua_config_f32_assignment_from_query(source: &str, field: &str) -> Option<f32> {
    lua_config_assignment_from_query(source, field, |value| {
        lua_static_number_assignment_value_from_query(
            source,
            value,
            lua_unsigned_number_literal_from_query,
        )
    })
    .and_then(|value| value.parse().ok())
}

#[allow(dead_code)]
fn lua_config_easing_assignment_from_query(
    source: &str,
    field: &str,
) -> Option<NativeEasingFunction> {
    lua_config_assignment_from_query(source, field, |value| {
        lua_static_easing_assignment_value_from_query(source, value)
    })
    .and_then(native_easing_lua_value_from_query)
}

#[allow(dead_code)]
fn lua_config_dimension_assignment_from_query(source: &str, field: &str) -> Option<String> {
    lua_config_string_assignment_from_query(source, field).or_else(|| {
        lua_config_assignment_from_query(source, field, |value| {
            lua_static_number_assignment_value_from_query(
                source,
                value,
                lua_signed_number_literal_from_query,
            )
        })
        .map(str::to_owned)
    })
}

#[allow(dead_code)]
fn lua_config_table_assignment_from_query<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    lua_config_assignment_from_query(source, field, lua_braced_table_literal_from_query)
}

fn lua_config_table_or_static_variable_assignment_from_query<'a>(
    source: &'a str,
    field: &str,
) -> Option<&'a str> {
    lua_config_assignment_from_query(source, field, |value| {
        let max_start = lua_source_slice_start_offset(source, value)?;
        lua_table_insert_value_table_from_query(source, value, max_start)
    })
}

struct LuaTableAssignmentWithMaxStart {
    value: String,
    max_start: usize,
}

struct LuaTableValueAssignment {
    value: String,
    variable: Option<String>,
}

struct LuaTableMapAssignment {
    value: String,
    variable: Option<String>,
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_config_table_assignment_with_insert_appends_with_max_start_from_query(
    source: &str,
    field: &str,
) -> Option<LuaTableAssignmentWithMaxStart> {
    if let Some(table) = lua_config_static_return_table_from_query(source) {
        let max_start = lua_source_slice_start_offset(source, table)?;
        let mut literal_from_query =
            |value| lua_table_insert_value_table_string_from_query(source, value, max_start);
        return lua_config_table_field_assignment_string_from_query_with_static_source(
            Some(LuaStaticSource { source, max_start }),
            table,
            field,
            &mut literal_from_query,
        )
        .map(|value| LuaTableAssignmentWithMaxStart { value, max_start });
    }

    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut selected = None;
    let mut selected_variable: Option<String> = None;

    for (index, character) in source.char_indices() {
        if let Some(end) = block_comment_end {
            if index < end {
                continue;
            }
            block_comment_end = None;
        }

        if let Some(end) = long_bracket_end {
            if index < end {
                continue;
            }
            long_bracket_end = None;
        }

        if line_comment {
            if character == '\n' {
                line_comment = false;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }

        if source[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index + 2..])
            {
                let content_and_rest = &source[index + 2 + content_start..];
                block_comment_end = Some(
                    content_and_rest
                        .find(&closing)
                        .map_or(source.len(), |close_index| {
                            index + 2 + content_start + close_index + closing.len()
                        }),
                );
                continue;
            }
            line_comment = true;
            continue;
        }

        match character {
            '\'' | '"' => {
                quote = Some(character);
                continue;
            }
            _ => {}
        }

        if character == '['
            && let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index..])
        {
            let content_and_rest = &source[index + content_start..];
            long_bracket_end = Some(
                content_and_rest
                    .find(&closing)
                    .map_or(source.len(), |close_index| {
                        index + content_start + close_index + closing.len()
                    }),
            );
            continue;
        }

        if lua_source_keyword_at(source, index, "function")
            || lua_source_keyword_at(source, index, "then")
            || lua_source_keyword_at(source, index, "do")
            || lua_source_keyword_at(source, index, "repeat")
        {
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }
        if lua_source_keyword_at(source, index, "end")
            || lua_source_keyword_at(source, index, "until")
        {
            lua_block_depth = lua_block_depth.saturating_sub(1);
            continue;
        }

        if lua_block_depth == 0 {
            let after_config = if lua_source_keyword_at(source, index, "local") {
                let rest = lua_trim_start_comments(source.get(index + "local".len()..)?)?;
                lua_config_receiver_prefix_rest(rest, receiver)
            } else {
                lua_config_receiver_prefix_rest(source.get(index..)?, receiver)
            };

            if let Some(after_config) = after_config {
                let after_config = lua_trim_start_comments(after_config)?;
                if let Some(after_assignment) = after_config.strip_prefix('=') {
                    let after_assignment = lua_trim_start_comments(after_assignment)?;
                    if let Some(table) = lua_table_insert_value_table_string_from_query(
                        source,
                        after_assignment,
                        index,
                    ) {
                        let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
                        if let Some(assignment) =
                            lua_config_table_value_field_assignment_from_table_query(
                                source, table, field, index,
                            )
                        {
                            selected = Some(LuaTableAssignmentWithMaxStart {
                                value: assignment.value,
                                max_start: index,
                            });
                            selected_variable = assignment.variable;
                        }
                    }
                }
            }
        }

        if source[index..].starts_with(field)
            && lua_config_assignment_field_has_boundaries(source, index, field)
            && lua_config_dot_assignment_has_receiver(source, index, receiver)
            && lua_block_depth == 0
        {
            let rest = lua_trim_start_comments(source.get(index + field.len()..)?)?;
            if let Some(rest) = rest.strip_prefix('=')
                && let Some(assignment) = lua_table_insert_value_table_assignment_from_query(
                    source,
                    lua_trim_start_comments(rest)?,
                    index,
                )
            {
                selected = Some(LuaTableAssignmentWithMaxStart {
                    value: assignment.value,
                    max_start: index,
                });
                selected_variable = assignment.variable;
            }
        }

        if character == '['
            && lua_block_depth == 0
            && let Some(rest) =
                lua_config_bracket_assignment_rest_from_query(source, index, receiver, field)
            && let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('=')
            && let Some(assignment) = lua_table_insert_value_table_assignment_from_query(
                source,
                lua_trim_start_comments(rest)?,
                index,
            )
        {
            selected = Some(LuaTableAssignmentWithMaxStart {
                value: assignment.value,
                max_start: index,
            });
            selected_variable = assignment.variable;
        }

        if lua_block_depth == 0
            && let Some(assignment) =
                lua_config_table_static_field_assignment_from_query(source, index, receiver, field)
        {
            selected = Some(
                lua_table_with_assigned_field(
                    selected.take().map(|assignment| assignment.value),
                    &assignment.key,
                    assignment.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
            continue;
        }

        if lua_block_depth == 0
            && let Some(assignment) =
                lua_config_table_indexed_field_assignment_from_query(source, index, receiver, field)
        {
            selected = Some(
                lua_table_with_index_field_assigned(
                    selected.take().map(|assignment| assignment.value),
                    assignment.index,
                    &assignment.key,
                    assignment.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
            continue;
        }

        if lua_block_depth == 0
            && let Some(insert) =
                lua_config_table_insert_append_value_from_query(source, index, receiver, field)
        {
            selected = Some(
                lua_table_with_inserted_field(
                    selected.take().map(|assignment| assignment.value),
                    insert.position,
                    &insert.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
        }

        if lua_block_depth == 0
            && let Some(assignment) = lua_config_table_index_or_append_assignment_from_query(
                source, index, receiver, field,
            )
        {
            selected = Some(
                lua_table_with_index_or_append_assigned_field(
                    selected.take().map(|assignment| assignment.value),
                    assignment.index,
                    &assignment.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
        }

        if lua_block_depth == 0 {
            lua_config_table_apply_selected_variable_mutation(
                source,
                index,
                &mut selected,
                &mut selected_variable,
            )?;
        }
    }

    selected
}

fn lua_config_table_apply_selected_variable_mutation(
    source: &str,
    index: usize,
    selected: &mut Option<LuaTableAssignmentWithMaxStart>,
    selected_variable: &mut Option<String>,
) -> Option<bool> {
    let Some(variable) = selected_variable.clone() else {
        return Some(false);
    };
    let rest = if lua_source_keyword_at(source, index, "local") {
        lua_trim_start_comments(source.get(index + "local".len()..)?)?
    } else {
        source.get(index..)?
    };
    if lua_static_table_variable_assignment_table_from_query(rest, &variable).is_some() {
        *selected_variable = None;
        return Some(true);
    }
    if let Some(assignment) =
        lua_static_table_variable_field_assignment_from_query(source, index, &variable)
    {
        *selected = Some(
            lua_table_with_assigned_field(
                selected.take().map(|assignment| assignment.value),
                &assignment.key,
                assignment.value,
            )
            .map(|value| LuaTableAssignmentWithMaxStart {
                value,
                max_start: index,
            })?,
        );
        return Some(true);
    }
    if let Some(assignment) =
        lua_static_table_variable_indexed_field_assignment_from_query(source, index, &variable)
    {
        *selected = Some(
            lua_table_with_index_field_assigned(
                selected.take().map(|assignment| assignment.value),
                assignment.index,
                &assignment.key,
                assignment.value,
            )
            .map(|value| LuaTableAssignmentWithMaxStart {
                value,
                max_start: index,
            })?,
        );
        return Some(true);
    }
    if let Some(assignment) = lua_static_table_variable_index_or_append_assignment_from_query(
        source, index, &variable,
    ) {
        *selected = Some(
            lua_table_with_index_or_append_assigned_field(
                selected.take().map(|assignment| assignment.value),
                assignment.index,
                &assignment.value,
            )
            .map(|value| LuaTableAssignmentWithMaxStart {
                value,
                max_start: index,
            })?,
        );
        return Some(true);
    }
    if let Some(insert) =
        lua_static_table_variable_insert_append_value_from_query(source, index, &variable)
    {
        *selected = Some(
            lua_table_with_inserted_field(
                selected.take().map(|assignment| assignment.value),
                insert.position,
                &insert.value,
            )
            .map(|value| LuaTableAssignmentWithMaxStart {
                value,
                max_start: index,
            })?,
        );
    }
    Some(false)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_config_table_map_assignment_with_field_mutations_from_query(
    source: &str,
    field: &str,
) -> Option<LuaTableAssignmentWithMaxStart> {
    if let Some(table) = lua_config_static_return_table_from_query(source) {
        let max_start = lua_source_slice_start_offset(source, table)?;
        let mut literal_from_query =
            |value| lua_table_insert_value_table_string_from_query(source, value, max_start);
        return lua_config_table_field_assignment_string_from_query_with_static_source(
            Some(LuaStaticSource { source, max_start }),
            table,
            field,
            &mut literal_from_query,
        )
        .map(|value| LuaTableAssignmentWithMaxStart { value, max_start });
    }

    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let mut selected = None;
    let mut selected_variable: Option<String> = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        let statement = source.get(start..)?;
        let after_receiver = if lua_source_keyword_at(source, start, "local") {
            let rest = lua_trim_start_comments(source.get(start + "local".len()..)?)?;
            lua_config_receiver_prefix_rest(rest, receiver)
        } else {
            lua_config_receiver_prefix_rest(statement, receiver)
        };

        if let Some(after_receiver) = after_receiver {
            let after_receiver = lua_trim_start_comments(after_receiver)?;
            if let Some(after_assignment) = after_receiver.strip_prefix('=') {
                let after_assignment = lua_trim_start_comments(after_assignment)?;
                if let Some(table) =
                    lua_table_map_value_table_string_from_query(source, after_assignment, start)
                {
                    let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
                    if let Some(assignment) = lua_config_table_map_field_assignment_from_table_query(
                        source, table, field, start,
                    ) {
                        selected = Some(LuaTableAssignmentWithMaxStart {
                            value: assignment.value,
                            max_start: start,
                        });
                        selected_variable = assignment.variable;
                    }
                }
            }

            if let Some(after_field) = lua_config_field_access_rest_from_query_with_static_key(
                source,
                after_receiver,
                field,
                start,
            ) {
                let after_field = lua_trim_start_comments(after_field)?;
                if let Some(after_assignment) = after_field.strip_prefix('=')
                    && let Some(assignment) = lua_table_map_assignment_from_query(
                        source,
                        lua_trim_start_comments(after_assignment)?,
                        start,
                    )
                {
                    selected = Some(LuaTableAssignmentWithMaxStart {
                        value: assignment.value,
                        max_start: start,
                    });
                    selected_variable = assignment.variable;
                    continue;
                }
            }
        }

        if let Some(assignment) =
            lua_config_table_map_field_assignment_from_query(source, start, receiver, field)
        {
            selected = Some(
                lua_table_with_assigned_field(
                    selected.take().map(|assignment| assignment.value),
                    &assignment.key,
                    assignment.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: start,
                })?,
            );
        }

        if let Some(variable) = selected_variable.clone() {
            let rest = if lua_source_keyword_at(source, start, "local") {
                lua_trim_start_comments(source.get(start + "local".len()..)?)?
            } else {
                source.get(start..)?
            };
            if lua_static_table_variable_assignment_table_from_query(rest, &variable).is_some() {
                selected_variable = None;
                continue;
            }

            if let Some(assignment) =
                lua_static_table_map_variable_field_assignment_from_query(source, start, &variable)
            {
                selected = Some(
                    lua_table_with_assigned_field(
                        selected.take().map(|assignment| assignment.value),
                        &assignment.key,
                        assignment.value,
                    )
                    .map(|value| LuaTableAssignmentWithMaxStart {
                        value,
                        max_start: start,
                    })?,
                );
            }
        }
    }

    selected
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_config_u32_array_assignment_with_insert_appends_with_max_start_from_query(
    source: &str,
    field: &str,
) -> Option<LuaTableAssignmentWithMaxStart> {
    if let Some(table) = lua_config_static_return_table_from_query(source) {
        let max_start = lua_source_slice_start_offset(source, table)?;
        let mut literal_from_query =
            |value| lua_u32_array_value_table_string_from_query(source, value, max_start);
        return lua_config_table_field_assignment_string_from_query_with_static_source(
            Some(LuaStaticSource { source, max_start }),
            table,
            field,
            &mut literal_from_query,
        )
        .map(|value| LuaTableAssignmentWithMaxStart { value, max_start });
    }

    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut selected = None;
    let mut selected_variable: Option<String> = None;

    for (index, character) in source.char_indices() {
        if let Some(end) = block_comment_end {
            if index < end {
                continue;
            }
            block_comment_end = None;
        }

        if let Some(end) = long_bracket_end {
            if index < end {
                continue;
            }
            long_bracket_end = None;
        }

        if line_comment {
            if character == '\n' {
                line_comment = false;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }

        if source[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index + 2..])
            {
                let content_and_rest = &source[index + 2 + content_start..];
                block_comment_end = Some(
                    content_and_rest
                        .find(&closing)
                        .map_or(source.len(), |close_index| {
                            index + 2 + content_start + close_index + closing.len()
                        }),
                );
                continue;
            }
            line_comment = true;
            continue;
        }

        match character {
            '\'' | '"' => {
                quote = Some(character);
                continue;
            }
            _ => {}
        }

        if character == '['
            && let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index..])
        {
            let content_and_rest = &source[index + content_start..];
            long_bracket_end = Some(
                content_and_rest
                    .find(&closing)
                    .map_or(source.len(), |close_index| {
                        index + content_start + close_index + closing.len()
                    }),
            );
            continue;
        }

        if lua_source_keyword_at(source, index, "function")
            || lua_source_keyword_at(source, index, "then")
            || lua_source_keyword_at(source, index, "do")
            || lua_source_keyword_at(source, index, "repeat")
        {
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }
        if lua_source_keyword_at(source, index, "end")
            || lua_source_keyword_at(source, index, "until")
        {
            lua_block_depth = lua_block_depth.saturating_sub(1);
            continue;
        }

        if lua_block_depth == 0 {
            let after_config = if lua_source_keyword_at(source, index, "local") {
                let rest = lua_trim_start_comments(source.get(index + "local".len()..)?)?;
                lua_config_receiver_prefix_rest(rest, receiver)
            } else {
                lua_config_receiver_prefix_rest(source.get(index..)?, receiver)
            };

            if let Some(after_config) = after_config {
                let after_config = lua_trim_start_comments(after_config)?;
                if let Some(after_assignment) = after_config.strip_prefix('=') {
                    let after_assignment = lua_trim_start_comments(after_assignment)?;
                    if let Some(table) =
                        lua_u32_array_value_table_string_from_query(source, after_assignment, index)
                    {
                        let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
                        if let Some(assignment) =
                            lua_config_u32_array_field_assignment_from_table_query(
                                source, table, field, index,
                            )
                        {
                            selected = Some(LuaTableAssignmentWithMaxStart {
                                value: assignment.value,
                                max_start: index,
                            });
                            selected_variable = assignment.variable;
                        }
                    }
                }
            }
        }

        if source[index..].starts_with(field)
            && lua_config_assignment_field_has_boundaries(source, index, field)
            && lua_config_dot_assignment_has_receiver(source, index, receiver)
            && lua_block_depth == 0
        {
            let rest = lua_trim_start_comments(source.get(index + field.len()..)?)?;
            if let Some(rest) = rest.strip_prefix('=')
                && let Some(assignment) = lua_u32_array_value_table_assignment_from_query(
                    source,
                    lua_trim_start_comments(rest)?,
                    index,
                )
            {
                selected = Some(LuaTableAssignmentWithMaxStart {
                    value: assignment.value,
                    max_start: index,
                });
                selected_variable = assignment.variable;
            }
        }

        if character == '['
            && lua_block_depth == 0
            && let Some(rest) =
                lua_config_bracket_assignment_rest_from_query(source, index, receiver, field)
            && let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('=')
            && let Some(assignment) = lua_u32_array_value_table_assignment_from_query(
                source,
                lua_trim_start_comments(rest)?,
                index,
            )
        {
            selected = Some(LuaTableAssignmentWithMaxStart {
                value: assignment.value,
                max_start: index,
            });
            selected_variable = assignment.variable;
        }

        if lua_block_depth == 0
            && let Some(insert) =
                lua_config_u32_array_insert_append_value_from_query(source, index, receiver, field)
        {
            selected = Some(
                lua_table_with_inserted_field(
                    selected.take().map(|assignment| assignment.value),
                    insert.position,
                    &insert.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
        }

        if lua_block_depth == 0
            && let Some(assignment) = lua_config_u32_array_index_or_append_assignment_from_query(
                source, index, receiver, field,
            )
        {
            selected = Some(
                lua_table_with_index_or_append_assigned_field(
                    selected.take().map(|assignment| assignment.value),
                    assignment.index,
                    assignment.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
        }

        if lua_block_depth == 0
            && let Some(variable) = selected_variable.clone()
        {
            let rest = if lua_source_keyword_at(source, index, "local") {
                lua_trim_start_comments(source.get(index + "local".len()..)?)?
            } else {
                source.get(index..)?
            };
            if lua_static_table_variable_assignment_table_from_query(rest, &variable).is_some() {
                selected_variable = None;
                continue;
            }

            if let Some(assignment) =
                lua_static_u32_array_variable_index_or_append_assignment_from_query(
                    source, index, &variable,
                )
            {
                selected = Some(
                    lua_table_with_index_or_append_assigned_field(
                        selected.take().map(|assignment| assignment.value),
                        assignment.index,
                        assignment.value,
                    )
                    .map(|value| LuaTableAssignmentWithMaxStart {
                        value,
                        max_start: index,
                    })?,
                );
                continue;
            }

            if let Some(insert) = lua_static_u32_array_variable_insert_append_value_from_query(
                source, index, &variable,
            ) {
                selected = Some(
                    lua_table_with_inserted_field(
                        selected.take().map(|assignment| assignment.value),
                        insert.position,
                        &insert.value,
                    )
                    .map(|value| LuaTableAssignmentWithMaxStart {
                        value,
                        max_start: index,
                    })?,
                );
            }
        }
    }

    selected
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_config_string_array_assignment_with_insert_appends_with_max_start_from_query(
    source: &str,
    field: &str,
) -> Option<LuaTableAssignmentWithMaxStart> {
    if let Some(table) = lua_config_static_return_table_from_query(source) {
        let max_start = lua_source_slice_start_offset(source, table)?;
        let mut literal_from_query =
            |value| lua_string_array_value_table_string_from_query(source, value, max_start);
        return lua_config_table_field_assignment_string_from_query_with_static_source(
            Some(LuaStaticSource { source, max_start }),
            table,
            field,
            &mut literal_from_query,
        )
        .map(|value| LuaTableAssignmentWithMaxStart { value, max_start });
    }

    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut selected = None;
    let mut selected_variable: Option<String> = None;

    for (index, character) in source.char_indices() {
        if let Some(end) = block_comment_end {
            if index < end {
                continue;
            }
            block_comment_end = None;
        }

        if let Some(end) = long_bracket_end {
            if index < end {
                continue;
            }
            long_bracket_end = None;
        }

        if line_comment {
            if character == '\n' {
                line_comment = false;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }

        if source[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index + 2..])
            {
                let content_and_rest = &source[index + 2 + content_start..];
                block_comment_end = Some(
                    content_and_rest
                        .find(&closing)
                        .map_or(source.len(), |close_index| {
                            index + 2 + content_start + close_index + closing.len()
                        }),
                );
                continue;
            }
            line_comment = true;
            continue;
        }

        match character {
            '\'' | '"' => {
                quote = Some(character);
                continue;
            }
            _ => {}
        }

        if character == '['
            && let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index..])
        {
            let content_and_rest = &source[index + content_start..];
            long_bracket_end = Some(
                content_and_rest
                    .find(&closing)
                    .map_or(source.len(), |close_index| {
                        index + content_start + close_index + closing.len()
                    }),
            );
            continue;
        }

        if lua_source_keyword_at(source, index, "function")
            || lua_source_keyword_at(source, index, "then")
            || lua_source_keyword_at(source, index, "do")
            || lua_source_keyword_at(source, index, "repeat")
        {
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }
        if lua_source_keyword_at(source, index, "end")
            || lua_source_keyword_at(source, index, "until")
        {
            lua_block_depth = lua_block_depth.saturating_sub(1);
            continue;
        }

        if lua_block_depth == 0 {
            let after_config = if lua_source_keyword_at(source, index, "local") {
                let rest = lua_trim_start_comments(source.get(index + "local".len()..)?)?;
                lua_config_receiver_prefix_rest(rest, receiver)
            } else {
                lua_config_receiver_prefix_rest(source.get(index..)?, receiver)
            };

            if let Some(after_config) = after_config {
                let after_config = lua_trim_start_comments(after_config)?;
                if let Some(after_assignment) = after_config.strip_prefix('=') {
                    let after_assignment = lua_trim_start_comments(after_assignment)?;
                    if let Some(table) = lua_string_array_value_table_string_from_query(
                        source,
                        after_assignment,
                        index,
                    ) {
                        let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
                        if let Some(assignment) =
                            lua_config_string_array_field_assignment_from_table_query(
                                source, table, field, index,
                            )
                        {
                            selected = Some(LuaTableAssignmentWithMaxStart {
                                value: assignment.value,
                                max_start: index,
                            });
                            selected_variable = assignment.variable;
                        }
                    }
                }
            }
        }

        if source[index..].starts_with(field)
            && lua_config_assignment_field_has_boundaries(source, index, field)
            && lua_config_dot_assignment_has_receiver(source, index, receiver)
            && lua_block_depth == 0
        {
            let rest = lua_trim_start_comments(source.get(index + field.len()..)?)?;
            if let Some(rest) = rest.strip_prefix('=')
                && let Some(assignment) = lua_string_array_value_table_assignment_from_query(
                    source,
                    lua_trim_start_comments(rest)?,
                    index,
                )
            {
                selected = Some(LuaTableAssignmentWithMaxStart {
                    value: assignment.value,
                    max_start: index,
                });
                selected_variable = assignment.variable;
            }
        }

        if character == '['
            && lua_block_depth == 0
            && let Some(rest) =
                lua_config_bracket_assignment_rest_from_query(source, index, receiver, field)
            && let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('=')
            && let Some(assignment) = lua_string_array_value_table_assignment_from_query(
                source,
                lua_trim_start_comments(rest)?,
                index,
            )
        {
            selected = Some(LuaTableAssignmentWithMaxStart {
                value: assignment.value,
                max_start: index,
            });
            selected_variable = assignment.variable;
        }

        if lua_block_depth == 0
            && let Some(insert) = lua_config_string_array_insert_append_value_from_query(
                source, index, receiver, field,
            )
        {
            selected = Some(
                lua_table_with_inserted_field(
                    selected.take().map(|assignment| assignment.value),
                    insert.position,
                    &insert.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
        }

        if lua_block_depth == 0
            && let Some(assignment) = lua_config_string_array_index_or_append_assignment_from_query(
                source, index, receiver, field,
            )
        {
            selected = Some(
                lua_table_with_index_or_append_assigned_field(
                    selected.take().map(|assignment| assignment.value),
                    assignment.index,
                    assignment.value,
                )
                .map(|value| LuaTableAssignmentWithMaxStart {
                    value,
                    max_start: index,
                })?,
            );
        }

        if lua_block_depth == 0
            && let Some(variable) = selected_variable.clone()
        {
            let rest = if lua_source_keyword_at(source, index, "local") {
                lua_trim_start_comments(source.get(index + "local".len()..)?)?
            } else {
                source.get(index..)?
            };
            if lua_static_table_variable_assignment_table_from_query(rest, &variable).is_some() {
                selected_variable = None;
                continue;
            }

            if let Some(assignment) =
                lua_static_string_array_variable_index_or_append_assignment_from_query(
                    source, index, &variable,
                )
            {
                selected = Some(
                    lua_table_with_index_or_append_assigned_field(
                        selected.take().map(|assignment| assignment.value),
                        assignment.index,
                        assignment.value,
                    )
                    .map(|value| LuaTableAssignmentWithMaxStart {
                        value,
                        max_start: index,
                    })?,
                );
                continue;
            }

            if let Some(insert) = lua_static_string_array_variable_insert_append_value_from_query(
                source, index, &variable,
            ) {
                selected = Some(
                    lua_table_with_inserted_field(
                        selected.take().map(|assignment| assignment.value),
                        insert.position,
                        &insert.value,
                    )
                    .map(|value| LuaTableAssignmentWithMaxStart {
                        value,
                        max_start: index,
                    })?,
                );
            }
        }
    }

    selected
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_config_key_tables_assignment_with_insert_appends_with_max_start_from_query(
    source: &str,
) -> Option<LuaTableAssignmentWithMaxStart> {
    if let Some(table) = lua_config_static_return_table_from_query(source) {
        let max_start = lua_source_slice_start_offset(source, table)?;
        let mut literal_from_query =
            |value| lua_key_tables_value_table_string_from_query(source, value, max_start);
        return lua_config_table_field_assignment_string_from_query(
            table,
            "key_tables",
            &mut literal_from_query,
        )
        .map(|value| LuaTableAssignmentWithMaxStart { value, max_start });
    }

    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut selected = None;
    let mut selected_variable: Option<String> = None;

    for (index, character) in source.char_indices() {
        if let Some(end) = block_comment_end {
            if index < end {
                continue;
            }
            block_comment_end = None;
        }

        if let Some(end) = long_bracket_end {
            if index < end {
                continue;
            }
            long_bracket_end = None;
        }

        if line_comment {
            if character == '\n' {
                line_comment = false;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }

        if source[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index + 2..])
            {
                let content_and_rest = &source[index + 2 + content_start..];
                block_comment_end = Some(
                    content_and_rest
                        .find(&closing)
                        .map_or(source.len(), |close_index| {
                            index + 2 + content_start + close_index + closing.len()
                        }),
                );
                continue;
            }
            line_comment = true;
            continue;
        }

        match character {
            '\'' | '"' => {
                quote = Some(character);
                continue;
            }
            _ => {}
        }

        if character == '['
            && let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&source[index..])
        {
            let content_and_rest = &source[index + content_start..];
            long_bracket_end = Some(
                content_and_rest
                    .find(&closing)
                    .map_or(source.len(), |close_index| {
                        index + content_start + close_index + closing.len()
                    }),
            );
            continue;
        }

        if lua_source_keyword_at(source, index, "function")
            || lua_source_keyword_at(source, index, "then")
            || lua_source_keyword_at(source, index, "do")
            || lua_source_keyword_at(source, index, "repeat")
        {
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }
        if lua_source_keyword_at(source, index, "end")
            || lua_source_keyword_at(source, index, "until")
        {
            lua_block_depth = lua_block_depth.saturating_sub(1);
            continue;
        }

        if lua_block_depth == 0 {
            let after_config = if lua_source_keyword_at(source, index, "local") {
                let rest = lua_trim_start_comments(source.get(index + "local".len()..)?)?;
                lua_config_receiver_prefix_rest(rest, receiver)
            } else {
                lua_config_receiver_prefix_rest(source.get(index..)?, receiver)
            };

            if let Some(after_config) = after_config {
                let after_config = lua_trim_start_comments(after_config)?;
                if let Some(after_assignment) = after_config.strip_prefix('=') {
                    let after_assignment = lua_trim_start_comments(after_assignment)?;
                    if let Some(table) = lua_table_insert_value_table_string_from_query(
                        source,
                        after_assignment,
                        index,
                    ) {
                        let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
                        let mut literal_from_query = |value| {
                            lua_key_tables_value_table_string_from_query(source, value, index)
                        };
                        if let Some(value) = lua_config_table_field_assignment_string_from_query(
                            table,
                            "key_tables",
                            &mut literal_from_query,
                        ) {
                            selected = Some(LuaTableAssignmentWithMaxStart {
                                value,
                                max_start: index,
                            });
                        }
                    }
                }
            }
        }

        if source[index..].starts_with("key_tables")
            && lua_config_assignment_field_has_boundaries(source, index, "key_tables")
            && lua_config_dot_assignment_has_receiver(source, index, receiver)
            && lua_block_depth == 0
        {
            let rest = lua_trim_start_comments(source.get(index + "key_tables".len()..)?)?;
            if let Some(rest) = rest.strip_prefix('=')
                && let Some(assignment) = lua_key_tables_value_table_assignment_from_query(
                    source,
                    lua_trim_start_comments(rest)?,
                    index,
                )
            {
                selected = Some(LuaTableAssignmentWithMaxStart {
                    value: assignment.value,
                    max_start: index,
                });
                selected_variable = assignment.variable;
            }
        }

        if character == '['
            && lua_block_depth == 0
            && let Some(rest) =
                lua_config_bracket_assignment_rest_from_query(source, index, receiver, "key_tables")
            && let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('=')
            && let Some(assignment) = lua_key_tables_value_table_assignment_from_query(
                source,
                lua_trim_start_comments(rest)?,
                index,
            )
        {
            selected = Some(LuaTableAssignmentWithMaxStart {
                value: assignment.value,
                max_start: index,
            });
            selected_variable = assignment.variable;
        }

        if lua_block_depth == 0
            && let Some((key_table_name, insert)) = lua_config_nested_table_insert_append_from_query(
                source,
                index,
                receiver,
                "key_tables",
            )
        {
            selected = Some(LuaTableAssignmentWithMaxStart {
                value: lua_key_tables_with_inserted_assignment(
                    selected.take().map(|assignment| assignment.value),
                    &key_table_name,
                    insert.position,
                    &insert.value,
                )?,
                max_start: index,
            });
        }

        if lua_block_depth == 0
            && let Some((key_table_name, assignment)) =
                lua_config_nested_key_table_indexed_field_assignment_from_query(
                    source,
                    index,
                    receiver,
                    "key_tables",
                )
        {
            selected = Some(LuaTableAssignmentWithMaxStart {
                value: lua_key_tables_with_index_field_assigned(
                    selected.take().map(|assignment| assignment.value),
                    &key_table_name,
                    assignment.index,
                    &assignment.key,
                    assignment.value,
                )?,
                max_start: index,
            });
        }

        if lua_block_depth == 0
            && let Some((key_table_name, assignment)) =
                lua_config_nested_key_table_index_or_append_assignment_from_query(
                    source,
                    index,
                    receiver,
                    "key_tables",
                )
        {
            selected = Some(LuaTableAssignmentWithMaxStart {
                value: lua_key_tables_with_index_or_append_assigned_assignment(
                    selected.take().map(|assignment| assignment.value),
                    &key_table_name,
                    assignment.index,
                    &assignment.value,
                )?,
                max_start: index,
            });
        }

        if lua_block_depth == 0 {
            lua_key_tables_apply_selected_variable_mutation(
                source,
                index,
                &mut selected,
                &mut selected_variable,
            )?;
        }
    }

    selected
}

fn lua_key_tables_apply_selected_variable_mutation(
    source: &str,
    index: usize,
    selected: &mut Option<LuaTableAssignmentWithMaxStart>,
    selected_variable: &mut Option<String>,
) -> Option<bool> {
    let Some(variable) = selected_variable.clone() else {
        return Some(false);
    };
    let rest = if lua_source_keyword_at(source, index, "local") {
        lua_trim_start_comments(source.get(index + "local".len()..)?)?
    } else {
        source.get(index..)?
    };
    if lua_static_table_variable_assignment_table_from_query(rest, &variable).is_some() {
        *selected_variable = None;
        return Some(true);
    }
    if let Some((key_table_name, table)) =
        lua_static_key_tables_variable_field_assignment_from_query(source, index, &variable)
    {
        *selected = Some(LuaTableAssignmentWithMaxStart {
            value: lua_key_tables_with_assigned_table(
                selected.take().map(|assignment| assignment.value),
                &key_table_name,
                table,
            )?,
            max_start: index,
        });
        return Some(true);
    }
    if let Some((key_table_name, assignment)) =
        lua_static_key_tables_variable_indexed_field_assignment_from_query(
            source, index, &variable,
        )
    {
        *selected = Some(LuaTableAssignmentWithMaxStart {
            value: lua_key_tables_with_index_field_assigned(
                selected.take().map(|assignment| assignment.value),
                &key_table_name,
                assignment.index,
                &assignment.key,
                assignment.value,
            )?,
            max_start: index,
        });
        return Some(true);
    }
    if let Some((key_table_name, assignment)) =
        lua_static_key_tables_variable_index_or_append_assignment_from_query(
            source, index, &variable,
        )
    {
        *selected = Some(LuaTableAssignmentWithMaxStart {
            value: lua_key_tables_with_index_or_append_assigned_assignment(
                selected.take().map(|assignment| assignment.value),
                &key_table_name,
                assignment.index,
                &assignment.value,
            )?,
            max_start: index,
        });
        return Some(true);
    }
    if let Some((key_table_name, insert)) =
        lua_static_nested_table_insert_append_from_query(source, index, &variable)
    {
        *selected = Some(LuaTableAssignmentWithMaxStart {
            value: lua_key_tables_with_inserted_assignment(
                selected.take().map(|assignment| assignment.value),
                &key_table_name,
                insert.position,
                &insert.value,
            )?,
            max_start: index,
        });
    }
    Some(false)
}

struct LuaTableInsertValue {
    position: Option<usize>,
    value: String,
}

struct LuaTableIndexAssignment<V> {
    index: usize,
    value: V,
}

struct LuaTableIndexOrAppendAssignment<V> {
    index: Option<usize>,
    value: V,
}

struct LuaTableIndexedFieldAssignment<'a> {
    index: usize,
    key: String,
    value: &'a str,
}

struct LuaTableMapFieldAssignment<'a> {
    key: String,
    value: &'a str,
}

struct LuaTableMapFieldPathAssignment<'a> {
    keys: Vec<String>,
    value: &'a str,
}

fn lua_config_table_map_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableMapFieldAssignment<'a>> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (key, rest) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        rest,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableMapFieldAssignment {
        key,
        value: lua_static_string_assignment_value_from_query(source, rest)?,
    })
}

fn lua_config_table_static_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableMapFieldAssignment<'a>> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (key, rest) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        rest,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableMapFieldAssignment {
        key,
        value: lua_static_table_field_assignment_value_from_query(source, rest, start).or_else(
            || {
                if field == "tab_bar_style" {
                    lua_top_level_statement_value_from_query(rest)
                } else {
                    None
                }
            },
        )?,
    })
}

fn lua_config_table_indexed_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableIndexedFieldAssignment<'a>> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let (index, rest) = lua_table_array_index_access_rest_from_query(rest)?;
    let (key, rest) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        rest,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableIndexedFieldAssignment {
        index,
        key,
        value: lua_top_level_statement_value_from_query(rest)?,
    })
}

fn lua_static_table_variable_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableMapFieldAssignment<'a>> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let (key, rest) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        after_variable,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableMapFieldAssignment {
        key,
        value: lua_top_level_statement_value_from_query(rest)?,
    })
}

fn lua_static_table_field_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<&'a str> {
    if let Some(value) = lua_braced_table_literal_from_query(query)
        .or_else(|| lua_quoted_string_literal_from_query(query))
        .or_else(|| lua_long_bracket_literal_from_query(query))
        .or_else(|| lua_bool_literal_from_query(query))
        .or_else(|| lua_signed_number_literal_from_query(query))
    {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)
        .or_else(|| {
            lua_static_number_variable_assignment_before_offset_from_query(
                source,
                variable,
                max_start,
                lua_signed_number_literal_from_query,
            )
        })
        .or_else(|| {
            lua_static_bool_variable_assignment_before_offset_from_query(
                source, variable, max_start,
            )
        })
}

fn lua_table_map_field_key_from_query(query: &str) -> Option<(String, &str)> {
    if let Some(rest) = query.strip_prefix('.') {
        let rest = lua_trim_start_comments(rest)?;
        let key = lua_identifier_literal_from_query(rest)?;
        return Some((key.to_owned(), rest.get(key.len()..)?));
    }

    let after_open = lua_trim_start_comments(query.strip_prefix('[')?)?;
    let key_literal = lua_quoted_string_literal_from_query(after_open)
        .or_else(|| lua_long_bracket_literal_from_query(after_open))?;
    let key = parse_maybe_quoted_query_text(key_literal)?;
    let rest = lua_trim_start_comments(after_open.get(key_literal.len()..)?)?;
    Some((key, rest.strip_prefix(']')?))
}

fn lua_table_map_field_key_from_query_with_static_source<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    query: &'a str,
) -> Option<(String, &'a str)> {
    if let Some(parsed) = lua_table_map_field_key_from_query(query) {
        return Some(parsed);
    }

    let static_source = static_source?;
    let after_open = lua_trim_start_comments(query.strip_prefix('[')?)?;
    let key = lua_identifier_literal_from_query(after_open)?;
    let rest = lua_trim_start_comments(after_open.get(key.len()..)?)?;
    let rest = rest.strip_prefix(']')?;
    let key = lua_static_string_assignment_value_before_offset_from_query(
        static_source.source,
        key,
        static_source.max_start,
    )
    .and_then(parse_maybe_quoted_query_text)?;
    Some((key, rest))
}

fn lua_table_map_field_key_from_query_with_static_sources<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    query: &'a str,
) -> Option<(String, &'a str)> {
    lua_table_map_field_key_from_query_with_static_source(static_source, query).or_else(|| {
        lua_table_map_field_key_from_query_with_static_source(outer_static_source, query)
    })
}

fn lua_table_array_index_access_rest_from_query(query: &str) -> Option<(usize, &str)> {
    let after_open = lua_trim_start_comments(query)?.strip_prefix('[')?;
    let after_open = lua_trim_start_comments(after_open)?;
    let literal = lua_unsigned_integer_literal_from_query(after_open)?;
    let index = literal.parse().ok()?;
    let rest = lua_trim_start_comments(after_open.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    Some((index, rest))
}

fn lua_table_array_index_access_rest_from_query_with_static_source<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    query: &'a str,
) -> Option<(usize, &'a str)> {
    if let Some(parsed) = lua_table_array_index_access_rest_from_query(query) {
        return Some(parsed);
    }

    let static_source = static_source?;
    let after_open = lua_trim_start_comments(query)?.strip_prefix('[')?;
    let after_open = lua_trim_start_comments(after_open)?;
    let variable = lua_identifier_literal_from_query(after_open)?;
    let rest = lua_trim_start_comments(after_open.get(variable.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    let index = lua_static_number_assignment_value_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
        lua_unsigned_integer_literal_from_query,
    )?
    .parse()
    .ok()?;
    Some((index, rest))
}

fn lua_config_table_insert_append_value_from_query(
    source: &str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableInsertValue> {
    if !lua_source_keyword_at(source, start, "table") {
        return None;
    }

    let rest = lua_trim_start_comments(source.get(start + "table".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('.')?)?;
    if !rest.starts_with("insert") || !lua_config_assignment_field_has_boundaries(rest, 0, "insert")
    {
        return None;
    }

    let rest = lua_trim_start_comments(rest.get("insert".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let after_receiver = lua_config_receiver_prefix_rest(rest, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    if let Some(value) =
        lua_table_insert_argument_value_table_string_from_query(source, rest, start)
    {
        return Some(LuaTableInsertValue {
            position: None,
            value,
        });
    }

    let position_literal = lua_unsigned_integer_literal_from_query(rest)?;
    let position = position_literal.parse().ok()?;
    let rest = lua_trim_start_comments(rest.get(position_literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    Some(LuaTableInsertValue {
        position: Some(position),
        value: lua_table_insert_argument_value_table_string_from_query(source, rest, start)?,
    })
}

fn lua_config_table_index_or_append_assignment_from_query(
    source: &str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableIndexOrAppendAssignment<String>> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    if let Some(assignment) = lua_table_index_assignment_value_from_query(source, rest, start) {
        return Some(LuaTableIndexOrAppendAssignment {
            index: Some(assignment.index),
            value: assignment.value,
        });
    }

    let after_open = lua_trim_start_comments(rest)?.strip_prefix('[')?;
    let after_hash = lua_trim_start_comments(after_open)?.strip_prefix('#')?;
    let after_hash = lua_trim_start_comments(after_hash)?;
    let after_receiver = lua_config_receiver_prefix_rest(after_hash, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    Some(LuaTableIndexOrAppendAssignment {
        index: None,
        value: lua_table_length_append_assignment_value_after_target_from_query(
            source, rest, start,
        )?,
    })
}

fn lua_config_u32_array_insert_append_value_from_query(
    source: &str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableInsertValue> {
    if !lua_source_keyword_at(source, start, "table") {
        return None;
    }

    let rest = lua_trim_start_comments(source.get(start + "table".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('.')?)?;
    if !rest.starts_with("insert") || !lua_config_assignment_field_has_boundaries(rest, 0, "insert")
    {
        return None;
    }

    let rest = lua_trim_start_comments(rest.get("insert".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let after_receiver = lua_config_receiver_prefix_rest(rest, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    if let Some(value) = lua_table_insert_value_u32_from_query(source, rest, start) {
        return Some(LuaTableInsertValue {
            position: None,
            value: value.to_owned(),
        });
    }

    let position_literal = lua_unsigned_integer_literal_from_query(rest)?;
    let position = position_literal.parse().ok()?;
    let rest = lua_trim_start_comments(rest.get(position_literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    Some(LuaTableInsertValue {
        position: Some(position),
        value: lua_table_insert_value_u32_from_query(source, rest, start)?.to_owned(),
    })
}

fn lua_config_u32_array_index_or_append_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableIndexOrAppendAssignment<&'a str>> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    if let Some(assignment) = lua_u32_array_index_assignment_value_from_query(source, rest, start) {
        return Some(LuaTableIndexOrAppendAssignment {
            index: Some(assignment.index),
            value: assignment.value,
        });
    }

    let after_open = lua_trim_start_comments(rest)?.strip_prefix('[')?;
    let after_hash = lua_trim_start_comments(after_open)?.strip_prefix('#')?;
    let after_hash = lua_trim_start_comments(after_hash)?;
    let after_receiver = lua_config_receiver_prefix_rest(after_hash, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    Some(LuaTableIndexOrAppendAssignment {
        index: None,
        value: lua_u32_array_length_append_assignment_value_after_target_from_query(
            source, rest, start,
        )?,
    })
}

fn lua_config_string_array_insert_append_value_from_query(
    source: &str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableInsertValue> {
    if !lua_source_keyword_at(source, start, "table") {
        return None;
    }

    let rest = lua_trim_start_comments(source.get(start + "table".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('.')?)?;
    if !rest.starts_with("insert") || !lua_config_assignment_field_has_boundaries(rest, 0, "insert")
    {
        return None;
    }

    let rest = lua_trim_start_comments(rest.get("insert".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let after_receiver = lua_config_receiver_prefix_rest(rest, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    if let Some(value) = lua_table_insert_value_string_from_query(source, rest, start) {
        return Some(LuaTableInsertValue {
            position: None,
            value: value.to_owned(),
        });
    }

    let position_literal = lua_unsigned_integer_literal_from_query(rest)?;
    let position = position_literal.parse().ok()?;
    let rest = lua_trim_start_comments(rest.get(position_literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    Some(LuaTableInsertValue {
        position: Some(position),
        value: lua_table_insert_value_string_from_query(source, rest, start)?.to_owned(),
    })
}

fn lua_config_string_array_index_or_append_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<LuaTableIndexOrAppendAssignment<&'a str>> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    if let Some(assignment) =
        lua_string_array_index_assignment_value_from_query(source, rest, start)
    {
        return Some(LuaTableIndexOrAppendAssignment {
            index: Some(assignment.index),
            value: assignment.value,
        });
    }

    let after_open = lua_trim_start_comments(rest)?.strip_prefix('[')?;
    let after_hash = lua_trim_start_comments(after_open)?.strip_prefix('#')?;
    let after_hash = lua_trim_start_comments(after_hash)?;
    let after_receiver = lua_config_receiver_prefix_rest(after_hash, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    Some(LuaTableIndexOrAppendAssignment {
        index: None,
        value: lua_string_array_length_append_assignment_value_after_target_from_query(
            source, rest, start,
        )?,
    })
}

fn lua_table_insert_value_table_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<&'a str> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    lua_static_table_variable_assignment_before_offset_from_query(source, variable, max_start)
}

fn lua_table_insert_value_table_string_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    lua_table_insert_value_table_assignment_from_query(source, query, max_start)
        .map(|assignment| assignment.value)
}

fn lua_table_insert_argument_value_table_string_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(value.to_owned());
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = lua_trim_start_comments(query.get(variable.len()..)?)?;
    if !rest.starts_with(')') {
        return None;
    }
    lua_static_table_variable_assignment_with_insert_appends_before_offset_from_query(
        source, variable, max_start,
    )
}

fn lua_table_insert_value_table_assignment_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<LuaTableValueAssignment> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(LuaTableValueAssignment {
            value: value.to_owned(),
            variable: None,
        });
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let value = lua_static_table_variable_assignment_with_insert_appends_before_offset_from_query(
        source, variable, max_start,
    )?;
    Some(LuaTableValueAssignment {
        value,
        variable: Some(variable.to_owned()),
    })
}

fn lua_table_map_value_table_string_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    lua_table_map_assignment_from_query(source, query, max_start).map(|assignment| assignment.value)
}

fn lua_table_map_assignment_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<LuaTableMapAssignment> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(LuaTableMapAssignment {
            value: value.to_owned(),
            variable: None,
        });
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let value =
        lua_static_table_map_variable_assignment_with_field_mutations_before_offset_from_query(
            source, variable, max_start,
        )?;
    Some(LuaTableMapAssignment {
        value,
        variable: Some(variable.to_owned()),
    })
}

fn lua_u32_array_value_table_string_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    lua_u32_array_value_table_assignment_from_query(source, query, max_start)
        .map(|assignment| assignment.value)
}

fn lua_u32_array_value_table_assignment_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<LuaTableValueAssignment> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(LuaTableValueAssignment {
            value: value.to_owned(),
            variable: None,
        });
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let value =
        lua_static_u32_array_variable_assignment_with_insert_appends_before_offset_from_query(
            source, variable, max_start,
        )?;
    Some(LuaTableValueAssignment {
        value,
        variable: Some(variable.to_owned()),
    })
}

fn lua_table_insert_value_u32_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<&'a str> {
    if let Some(value) = lua_unsigned_u32_literal_from_query(query) {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    lua_static_number_variable_assignment_before_offset_from_query(
        source,
        variable,
        max_start,
        lua_unsigned_u32_literal_from_query,
    )
}

fn lua_string_array_value_table_string_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    lua_string_array_value_table_assignment_from_query(source, query, max_start)
        .map(|assignment| assignment.value)
}

fn lua_string_array_value_table_assignment_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<LuaTableValueAssignment> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(LuaTableValueAssignment {
            value: value.to_owned(),
            variable: None,
        });
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let value =
        lua_static_string_array_variable_assignment_with_insert_appends_before_offset_from_query(
            source, variable, max_start,
        )?;
    Some(LuaTableValueAssignment {
        value,
        variable: Some(variable.to_owned()),
    })
}

fn lua_table_insert_value_string_from_query<'a>(
    source: &str,
    query: &'a str,
    max_start: usize,
) -> Option<&'a str> {
    if let Some(value) = lua_quoted_string_literal_from_query(query)
        .or_else(|| lua_long_bracket_literal_from_query(query))
    {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)?;
    Some(variable)
}

fn lua_key_tables_value_table_string_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    lua_key_tables_value_table_assignment_from_query(source, query, max_start)
        .map(|assignment| assignment.value)
}

fn lua_key_tables_value_table_assignment_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<LuaTableValueAssignment> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(LuaTableValueAssignment {
            value: value.to_owned(),
            variable: None,
        });
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let value =
        lua_static_key_tables_variable_assignment_with_insert_appends_before_offset_from_query(
            source, variable, max_start,
        )?;
    Some(LuaTableValueAssignment {
        value,
        variable: Some(variable.to_owned()),
    })
}

fn lua_static_key_tables_variable_assignment_with_insert_appends_before_offset_from_query(
    source: &str,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        if let Some(table) = lua_static_table_variable_assignment_table_from_query(rest, variable) {
            selected = Some(table.to_owned());
            continue;
        }

        if let Some((key_table_name, table)) =
            lua_static_key_tables_variable_field_assignment_from_query(source, start, variable)
        {
            selected = Some(lua_key_tables_with_assigned_table(
                selected.take(),
                &key_table_name,
                table,
            )?);
            continue;
        }

        if let Some((key_table_name, assignment)) =
            lua_static_key_tables_variable_indexed_field_assignment_from_query(
                source, start, variable,
            )
        {
            selected = Some(lua_key_tables_with_index_field_assigned(
                selected.take(),
                &key_table_name,
                assignment.index,
                &assignment.key,
                assignment.value,
            )?);
            continue;
        }

        if let Some((key_table_name, assignment)) =
            lua_static_key_tables_variable_index_or_append_assignment_from_query(
                source, start, variable,
            )
        {
            selected = Some(lua_key_tables_with_index_or_append_assigned_assignment(
                selected.take(),
                &key_table_name,
                assignment.index,
                &assignment.value,
            )?);
            continue;
        }

        if let Some((key_table_name, insert)) =
            lua_static_nested_table_insert_append_from_query(source, start, variable)
        {
            selected = Some(lua_key_tables_with_inserted_assignment(
                selected.take(),
                &key_table_name,
                insert.position,
                &insert.value,
            )?);
        }
    }

    selected
}

fn lua_static_table_variable_assignment_with_insert_appends_before_offset_from_query(
    source: &str,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        if let Some(table) = lua_static_table_variable_assignment_table_from_query(rest, variable) {
            selected = Some(table.to_owned());
            continue;
        }

        if let Some(assignment) =
            lua_static_table_variable_field_assignment_from_query(source, start, variable)
        {
            selected = Some(lua_table_with_assigned_field(
                selected.take(),
                &assignment.key,
                assignment.value,
            )?);
            continue;
        }

        if let Some(assignment) =
            lua_static_table_variable_indexed_field_assignment_from_query(source, start, variable)
        {
            selected = Some(lua_table_with_index_field_assigned(
                selected.take(),
                assignment.index,
                &assignment.key,
                assignment.value,
            )?);
            continue;
        }

        if let Some(assignment) =
            lua_static_table_variable_index_or_append_assignment_from_query(source, start, variable)
        {
            selected = Some(lua_table_with_index_or_append_assigned_field(
                selected.take(),
                assignment.index,
                &assignment.value,
            )?);
            continue;
        }

        if let Some(insert) =
            lua_static_table_variable_insert_append_value_from_query(source, start, variable)
        {
            selected = Some(lua_table_with_inserted_field(
                selected.take(),
                insert.position,
                &insert.value,
            )?);
        }
    }

    selected
}

fn lua_static_table_map_variable_assignment_with_field_mutations_before_offset_from_query(
    source: &str,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        if let Some(table) = lua_static_table_variable_assignment_table_from_query(rest, variable) {
            selected = Some(table.to_owned());
            continue;
        }

        if let Some(assignment) =
            lua_static_table_map_variable_field_assignment_from_query(source, start, variable)
        {
            selected = Some(lua_table_with_assigned_field(
                selected.take(),
                &assignment.key,
                assignment.value,
            )?);
        }
    }

    selected
}

fn lua_static_table_map_variable_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableMapFieldAssignment<'a>> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let after_variable = lua_trim_start_comments(after_variable)?;
    let (key, rest) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        after_variable,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableMapFieldAssignment {
        key,
        value: lua_static_string_assignment_value_from_query(source, rest)?,
    })
}

fn lua_static_string_array_variable_assignment_with_insert_appends_before_offset_from_query(
    source: &str,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        if let Some(table) = lua_static_table_variable_assignment_table_from_query(rest, variable) {
            selected = Some(table.to_owned());
            continue;
        }

        if let Some(assignment) =
            lua_static_string_array_variable_index_or_append_assignment_from_query(
                source, start, variable,
            )
        {
            selected = Some(lua_table_with_index_or_append_assigned_field(
                selected.take(),
                assignment.index,
                assignment.value,
            )?);
            continue;
        }

        if let Some(insert) =
            lua_static_string_array_variable_insert_append_value_from_query(source, start, variable)
        {
            selected = Some(lua_table_with_inserted_field(
                selected.take(),
                insert.position,
                &insert.value,
            )?);
        }
    }

    selected
}

fn lua_static_u32_array_variable_assignment_with_insert_appends_before_offset_from_query(
    source: &str,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        if let Some(table) = lua_static_table_variable_assignment_table_from_query(rest, variable) {
            selected = Some(table.to_owned());
            continue;
        }

        if let Some(assignment) =
            lua_static_u32_array_variable_index_or_append_assignment_from_query(
                source, start, variable,
            )
        {
            selected = Some(lua_table_with_index_or_append_assigned_field(
                selected.take(),
                assignment.index,
                assignment.value,
            )?);
            continue;
        }

        if let Some(insert) =
            lua_static_u32_array_variable_insert_append_value_from_query(source, start, variable)
        {
            selected = Some(lua_table_with_inserted_field(
                selected.take(),
                insert.position,
                &insert.value,
            )?);
        }
    }

    selected
}

fn lua_static_u32_array_variable_insert_append_value_from_query(
    source: &str,
    start: usize,
    variable: &str,
) -> Option<LuaTableInsertValue> {
    if !lua_source_keyword_at(source, start, "table") {
        return None;
    }

    let rest = lua_trim_start_comments(source.get(start + "table".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('.')?)?;
    if !rest.starts_with("insert") || !lua_config_assignment_field_has_boundaries(rest, 0, "insert")
    {
        return None;
    }

    let rest = lua_trim_start_comments(rest.get("insert".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let after_variable = rest.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let rest = lua_trim_start_comments(after_variable)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    if let Some(value) = lua_table_insert_value_u32_from_query(source, rest, start) {
        return Some(LuaTableInsertValue {
            position: None,
            value: value.to_owned(),
        });
    }

    let position_literal = lua_unsigned_integer_literal_from_query(rest)?;
    let position = position_literal.parse().ok()?;
    let rest = lua_trim_start_comments(rest.get(position_literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    Some(LuaTableInsertValue {
        position: Some(position),
        value: lua_table_insert_value_u32_from_query(source, rest, start)?.to_owned(),
    })
}

fn lua_static_u32_array_variable_index_or_append_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexOrAppendAssignment<&'a str>> {
    if let Some(assignment) =
        lua_static_u32_array_variable_index_assignment_from_query(source, start, variable)
    {
        return Some(LuaTableIndexOrAppendAssignment {
            index: Some(assignment.index),
            value: assignment.value,
        });
    }

    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let after_open = lua_trim_start_comments(after_variable)?.strip_prefix('[')?;
    let after_hash = lua_trim_start_comments(after_open)?.strip_prefix('#')?;
    let after_hash = lua_trim_start_comments(after_hash)?;
    let rest = after_hash.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    Some(LuaTableIndexOrAppendAssignment {
        index: None,
        value: lua_u32_array_length_append_assignment_value_after_target_from_query(
            source, rest, start,
        )?,
    })
}

fn lua_static_u32_array_variable_index_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexAssignment<&'a str>> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    lua_u32_array_index_assignment_value_from_query(source, after_variable, start)
}

fn lua_static_string_array_variable_insert_append_value_from_query(
    source: &str,
    start: usize,
    variable: &str,
) -> Option<LuaTableInsertValue> {
    if !lua_source_keyword_at(source, start, "table") {
        return None;
    }

    let rest = lua_trim_start_comments(source.get(start + "table".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('.')?)?;
    if !rest.starts_with("insert") || !lua_config_assignment_field_has_boundaries(rest, 0, "insert")
    {
        return None;
    }

    let rest = lua_trim_start_comments(rest.get("insert".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let after_variable = rest.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let rest = lua_trim_start_comments(after_variable)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    if let Some(value) = lua_table_insert_value_string_from_query(source, rest, start) {
        return Some(LuaTableInsertValue {
            position: None,
            value: value.to_owned(),
        });
    }

    let position_literal = lua_unsigned_integer_literal_from_query(rest)?;
    let position = position_literal.parse().ok()?;
    let rest = lua_trim_start_comments(rest.get(position_literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    Some(LuaTableInsertValue {
        position: Some(position),
        value: lua_table_insert_value_string_from_query(source, rest, start)?.to_owned(),
    })
}

fn lua_static_string_array_variable_index_or_append_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexOrAppendAssignment<&'a str>> {
    if let Some(assignment) =
        lua_static_string_array_variable_index_assignment_from_query(source, start, variable)
    {
        return Some(LuaTableIndexOrAppendAssignment {
            index: Some(assignment.index),
            value: assignment.value,
        });
    }

    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let after_open = lua_trim_start_comments(after_variable)?.strip_prefix('[')?;
    let after_hash = lua_trim_start_comments(after_open)?.strip_prefix('#')?;
    let after_hash = lua_trim_start_comments(after_hash)?;
    let rest = after_hash.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    Some(LuaTableIndexOrAppendAssignment {
        index: None,
        value: lua_string_array_length_append_assignment_value_after_target_from_query(
            source, rest, start,
        )?,
    })
}

fn lua_static_string_array_variable_index_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexAssignment<&'a str>> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    lua_string_array_index_assignment_value_from_query(source, after_variable, start)
}

fn lua_static_nested_table_insert_append_from_query(
    source: &str,
    start: usize,
    variable: &str,
) -> Option<(String, LuaTableInsertValue)> {
    if !lua_source_keyword_at(source, start, "table") {
        return None;
    }

    let rest = lua_trim_start_comments(source.get(start + "table".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('.')?)?;
    if !rest.starts_with("insert") || !lua_config_assignment_field_has_boundaries(rest, 0, "insert")
    {
        return None;
    }

    let rest = lua_trim_start_comments(rest.get("insert".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let after_variable = rest.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let (key_table_name, rest) =
        lua_nested_table_insert_key_from_query(source, after_variable, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    if let Some(value) =
        lua_table_insert_argument_value_table_string_from_query(source, rest, start)
    {
        return Some((
            key_table_name,
            LuaTableInsertValue {
                position: None,
                value,
            },
        ));
    }

    let position_literal = lua_unsigned_integer_literal_from_query(rest)?;
    let position = position_literal.parse().ok()?;
    let rest = lua_trim_start_comments(rest.get(position_literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    Some((
        key_table_name,
        LuaTableInsertValue {
            position: Some(position),
            value: lua_table_insert_argument_value_table_string_from_query(source, rest, start)?,
        },
    ))
}

fn lua_static_key_tables_variable_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<(String, &'a str)> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let (key_table_name, rest) =
        lua_nested_table_insert_key_from_query(source, after_variable, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    let table = lua_table_insert_value_table_from_query(source, rest, start)?;
    Some((key_table_name, table))
}

fn lua_static_key_tables_variable_index_or_append_assignment_from_query(
    source: &str,
    start: usize,
    variable: &str,
) -> Option<(String, LuaTableIndexOrAppendAssignment<String>)> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let (key_table_name, rest) =
        lua_nested_table_insert_key_from_query(source, after_variable, start)?;
    if let Some(assignment) = lua_table_index_assignment_value_from_query(source, rest, start) {
        return Some((
            key_table_name,
            LuaTableIndexOrAppendAssignment {
                index: Some(assignment.index),
                value: assignment.value,
            },
        ));
    }

    let assignment = lua_static_nested_table_length_append_assignment_from_query(
        source,
        rest,
        start,
        variable,
        &key_table_name,
    )?;
    Some((key_table_name, assignment))
}
