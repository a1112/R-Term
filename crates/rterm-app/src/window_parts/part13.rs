fn spawn_command_table_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    allow_position: bool,
) -> Option<WindowSpawnCommandQueryOptions> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut options = WindowSpawnCommandQueryOptions::default();
    let mut label = None;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = value.trim();
        if key.eq_ignore_ascii_case("args") {
            return None;
        } else if key.eq_ignore_ascii_case("cwd") {
            if options.cwd.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            options.cwd = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("label") {
            if label.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            label = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("set_environment_variables")
            || key.eq_ignore_ascii_case("set-environment-variables")
        {
            if !options.environment.is_empty() {
                return None;
            }
            options.environment =
                split_lua_table_environment_from_query_with_static_source(static_source, value)?;
        } else if key.eq_ignore_ascii_case("domain") {
            if options.domain.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            options.domain = Some(spawn_command_domain_from_query(&value)?);
        } else if key.eq_ignore_ascii_case("position") {
            if !allow_position || options.window_position.is_some() {
                return None;
            }
            options.window_position = Some(
                spawn_command_window_position_value_from_query_with_static_source(
                    static_source,
                    value,
                )
                .ok()?,
            );
        } else {
            return None;
        }
    }
    let _ = label;
    (options.cwd.is_some()
        || !options.environment.is_empty()
        || options.domain.is_some()
        || options.window_position.is_some())
    .then_some(options)
}
fn split_lua_table_environment_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<BTreeMap<String, String>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut environment = BTreeMap::new();
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = parse_maybe_static_query_text(static_source, value.trim())?;
        environment.insert(name, value);
    }
    Some(environment)
}

fn native_visual_bell_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: Option<usize>,
) -> Option<NativeVisualBell> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = max_start.map(|max_start| LuaStaticSource { source, max_start });
    let mut visual_bell = NativeVisualBell::default();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "fade_in_duration_ms" => {
                visual_bell.fade_in_duration_ms = lua_static_number_assignment_value_from_query(
                    source,
                    value,
                    lua_unsigned_integer_literal_from_query,
                )?
                .parse()
                .ok()?;
            }
            "fade_out_duration_ms" => {
                visual_bell.fade_out_duration_ms = lua_static_number_assignment_value_from_query(
                    source,
                    value,
                    lua_unsigned_integer_literal_from_query,
                )?
                .parse()
                .ok()?;
            }
            "fade_in_function" => {
                let value = lua_static_easing_assignment_value_from_query(source, value)?;
                visual_bell.fade_in_function = native_easing_lua_value_from_query(value)?;
            }
            "fade_out_function" => {
                let value = lua_static_easing_assignment_value_from_query(source, value)?;
                visual_bell.fade_out_function = native_easing_lua_value_from_query(value)?;
            }
            "target" => {
                let value = lua_static_string_assignment_value_from_query(source, value)?;
                let value = parse_maybe_quoted_query_text(value)?;
                visual_bell.target = NativeVisualBellTarget::parse(&value)?;
            }
            _ => return None,
        }
    }

    Some(visual_bell)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn visual_bell_color_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Option<Color>> {
    color_lua_table_field_from_query_with_static_source(static_source, value, "visual_bell")
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn tab_bar_background_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Option<Color>> {
    tab_bar_color_lua_table_field_from_query(static_source, value, "background")
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn tab_bar_inactive_tab_edge_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Option<Color>> {
    tab_bar_color_lua_table_field_from_query(static_source, value, "inactive_tab_edge")
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn tab_bar_color_lua_table_field_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    field_name: &str,
) -> Option<Option<Color>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut color = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != "tab_bar" {
            continue;
        }
        if color.is_some() {
            return None;
        }
        color = color_lua_table_field_from_query_with_static_source(
            static_source,
            value.trim(),
            field_name,
        )?;
    }

    Some(color)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn tab_bar_item_colors_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    item_name: &str,
) -> Option<Option<NativeTabBarItemColors>> {
    let tab_bar =
        lua_table_field_value_from_query_with_static_source(static_source, value, "tab_bar")?;
    let Some(tab_bar) = tab_bar else {
        return Some(None);
    };
    let item =
        lua_table_field_value_from_query_with_static_source(static_source, tab_bar, item_name)?;
    let Some(item) = item else {
        return Some(None);
    };

    let mut colors = NativeTabBarItemColors::default();
    for field in
        split_lua_table_top_level_fields(item.trim().strip_prefix('{')?.strip_suffix('}')?.trim())?
    {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "fg_color" => {
                let value = parse_maybe_static_query_text(static_source, value)?;
                colors.fg_color = Some(lua_opaque_color_from_query_with_static_source(
                    static_source,
                    &value,
                )?);
            }
            "bg_color" => {
                let value = parse_maybe_static_query_text(static_source, value)?;
                colors.bg_color = Some(lua_opaque_color_from_query_with_static_source(
                    static_source,
                    &value,
                )?);
            }
            "intensity" => {
                colors.intensity = Some(tab_bar_item_intensity_from_query(
                    &parse_maybe_static_query_text(static_source, value)?,
                )?);
            }
            "underline" => {
                colors.underline = Some(tab_bar_item_underline_from_query(
                    &parse_maybe_static_query_text(static_source, value)?,
                )?);
            }
            "italic" => colors.italic = Some(parse_maybe_static_query_bool(static_source, value)?),
            "strikethrough" => {
                colors.strikethrough = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            _ => {}
        }
    }

    Some(
        (colors.fg_color.is_some()
            || colors.bg_color.is_some()
            || colors.intensity.is_some()
            || colors.underline.is_some()
            || colors.italic.is_some()
            || colors.strikethrough.is_some())
        .then_some(colors),
    )
}

fn tab_bar_item_intensity_from_query(value: &str) -> Option<NativeFormatIntensity> {
    match value {
        "Normal" => Some(NativeFormatIntensity::Normal),
        "Bold" => Some(NativeFormatIntensity::Bold),
        "Half" => Some(NativeFormatIntensity::Half),
        _ => None,
    }
}

fn tab_bar_item_underline_from_query(value: &str) -> Option<NativeFormatUnderline> {
    match value {
        "None" => Some(NativeFormatUnderline::None),
        "Single" => Some(NativeFormatUnderline::Single),
        "Double" => Some(NativeFormatUnderline::Double),
        "Curly" => Some(NativeFormatUnderline::Curly),
        "Dotted" => Some(NativeFormatUnderline::Dotted),
        "Dashed" => Some(NativeFormatUnderline::Dashed),
        _ => None,
    }
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn native_tab_bar_style_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Option<NativeTabBarStyle>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut style = NativeTabBarStyle::default();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let items = native_format_items_from_wezterm_format_query_with_static_sources(
            static_source,
            None,
            value.trim(),
        )?;
        match key.as_str() {
            "active_tab_left" => assign_tab_bar_style_edge(&mut style.active_tab_left, items)?,
            "active_tab_right" => assign_tab_bar_style_edge(&mut style.active_tab_right, items)?,
            "inactive_tab_left" => assign_tab_bar_style_edge(&mut style.inactive_tab_left, items)?,
            "inactive_tab_right" => {
                assign_tab_bar_style_edge(&mut style.inactive_tab_right, items)?;
            }
            "inactive_tab_hover_left" => {
                assign_tab_bar_style_edge(&mut style.inactive_tab_hover_left, items)?;
            }
            "inactive_tab_hover_right" => {
                assign_tab_bar_style_edge(&mut style.inactive_tab_hover_right, items)?;
            }
            "new_tab" => assign_tab_bar_style_edge(&mut style.new_tab, items)?,
            "new_tab_hover" => assign_tab_bar_style_edge(&mut style.new_tab_hover, items)?,
            "new_tab_left" => assign_tab_bar_style_edge(&mut style.new_tab_left, items)?,
            "new_tab_right" => assign_tab_bar_style_edge(&mut style.new_tab_right, items)?,
            "new_tab_hover_left" => {
                assign_tab_bar_style_edge(&mut style.new_tab_hover_left, items)?;
            }
            "new_tab_hover_right" => {
                assign_tab_bar_style_edge(&mut style.new_tab_hover_right, items)?;
            }
            "window_hide" => assign_tab_bar_style_edge(&mut style.window_hide, items)?,
            "window_hide_hover" => {
                assign_tab_bar_style_edge(&mut style.window_hide_hover, items)?;
            }
            "window_maximize" => assign_tab_bar_style_edge(&mut style.window_maximize, items)?,
            "window_maximize_hover" => {
                assign_tab_bar_style_edge(&mut style.window_maximize_hover, items)?;
            }
            "window_close" => assign_tab_bar_style_edge(&mut style.window_close, items)?,
            "window_close_hover" => {
                assign_tab_bar_style_edge(&mut style.window_close_hover, items)?;
            }
            _ => {}
        }
    }

    Some((!style.is_empty()).then_some(style))
}

fn assign_tab_bar_style_edge(
    target: &mut Option<Vec<NativeFormatItem>>,
    items: Vec<NativeFormatItem>,
) -> Option<()> {
    if target.is_some() {
        return None;
    }
    *target = Some(items);
    Some(())
}

fn native_format_items_from_wezterm_format_query(value: &str) -> Option<Vec<NativeFormatItem>> {
    let table = wezterm_format_table_argument_from_query(value)?;
    native_format_items_from_lua_format_items_table_query(table)
}

fn native_format_items_from_wezterm_format_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<NativeFormatItem>> {
    let resolved_value = static_source
        .and_then(|static_source| {
            lua_static_wezterm_format_alias_query_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
        .or_else(|| {
            outer_static_source.and_then(|outer_static_source| {
                lua_static_wezterm_format_alias_query_from_query(
                    outer_static_source.source,
                    value,
                    outer_static_source.max_start,
                )
            })
        });
    let value = resolved_value.as_deref().unwrap_or(value);
    let table = wezterm_format_table_argument_from_query(value)?;
    if let Some(items) = native_format_items_from_lua_format_items_table_query_with_static_sources(
        static_source,
        outer_static_source,
        table,
    ) {
        return Some(items);
    }

    let variable = lua_identifier_literal_from_query(table)?;
    let rest = table.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    if let Some(static_source) = static_source
        && let Some(items) = native_format_items_from_static_lua_table_variable(
            static_source,
            outer_static_source,
            variable,
        )
    {
        return items;
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(items) =
            native_format_items_from_static_lua_table_variable(outer_static_source, None, variable)
    {
        return items;
    }

    None
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn native_format_items_from_static_lua_table_variable(
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    variable: &str,
) -> Option<Option<Vec<NativeFormatItem>>> {
    let _shadowing_value = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )?;
    let value = lua_format_items_table_variable_with_insert_appends_before_offset(
        static_source.source,
        outer_static_source,
        variable,
        static_source.max_start,
    )?;
    let items = native_format_items_from_lua_format_items_table_query_with_static_sources(
        Some(static_source),
        outer_static_source,
        &value,
    );
    Some(items)
}

fn lua_format_items_table_variable_with_insert_appends_before_offset(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
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
            lua_static_format_items_table_variable_index_or_append_assignment_from_query(
                source,
                outer_static_source,
                start,
                variable,
            )
        {
            selected = Some(lua_table_with_index_or_append_assigned_field(
                selected.take(),
                assignment.index,
                &assignment.value,
            )?);
            continue;
        }

        if let Some(insert) = lua_static_format_items_table_variable_insert_append_value_from_query(
            source,
            outer_static_source,
            start,
            variable,
        ) {
            selected = Some(lua_table_with_inserted_field(
                selected.take(),
                insert.position,
                &insert.value,
            )?);
        }
    }

    selected
}

fn lua_static_format_items_table_variable_index_or_append_assignment_from_query(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexOrAppendAssignment<String>> {
    if let Some(assignment) = lua_static_format_items_table_variable_index_assignment_from_query(
        source,
        outer_static_source,
        start,
        variable,
    ) {
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
        value: lua_format_item_length_append_assignment_value_after_target_from_query(
            source,
            outer_static_source,
            rest,
            start,
        )?,
    })
}

fn lua_static_format_items_table_variable_index_assignment_from_query(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexAssignment<String>> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    lua_format_item_index_assignment_value_from_query(
        source,
        outer_static_source,
        after_variable,
        start,
    )
}

fn lua_format_item_index_assignment_value_from_query(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    max_start: usize,
) -> Option<LuaTableIndexAssignment<String>> {
    let after_open = lua_trim_start_comments(query)?.strip_prefix('[')?;
    let after_open = lua_trim_start_comments(after_open)?;
    let literal = lua_unsigned_integer_literal_from_query(after_open)?;
    let index = literal.parse().ok()?;
    let rest = lua_trim_start_comments(after_open.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableIndexAssignment {
        index,
        value: lua_format_item_assignment_value_from_query(
            source,
            outer_static_source,
            rest,
            max_start,
        )?,
    })
}

fn lua_format_item_length_append_assignment_value_after_target_from_query(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let rest = lua_trim_start_comments(query)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('+')?)?;
    let literal = lua_unsigned_integer_literal_from_query(rest)?;
    if literal != "1" {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    lua_format_item_assignment_value_from_query(source, outer_static_source, rest, max_start)
}

fn lua_format_item_assignment_value_from_query(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    max_start: usize,
) -> Option<String> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(value.to_owned());
    }

    if let Some(value) = lua_quoted_string_literal_from_query(query)
        .or_else(|| lua_long_bracket_literal_from_query(query))
    {
        let rest = query.get(value.len()..)?;
        if lua_static_identifier_value_rest_is_statement_end(rest) {
            return Some(value.to_owned());
        }
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_format_item_table_variable_assignment_from_sources(
        source,
        outer_static_source,
        variable,
        max_start,
    )
    .or_else(|| {
        lua_format_item_string_variable_assignment_from_sources(
            source,
            outer_static_source,
            variable,
            max_start,
        )
    })
}

fn lua_format_item_table_variable_assignment_from_sources(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    lua_format_item_table_variable_assignment_from_sources_with_depth(
        source,
        outer_static_source,
        variable,
        max_start,
        0,
    )
}

fn lua_format_item_table_variable_assignment_from_sources_with_depth(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    variable: &str,
    max_start: usize,
    depth: usize,
) -> Option<String> {
    if depth > 8 {
        return None;
    }

    if let Some(value) =
        lua_static_table_variable_assignment_with_insert_appends_before_offset_from_query(
            source, variable, max_start,
        )
    {
        return Some(value);
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) =
            lua_static_table_variable_assignment_with_insert_appends_before_offset_from_query(
                outer_static_source.source,
                variable,
                outer_static_source.max_start,
            )
    {
        return Some(value);
    }

    let alias = lua_static_expression_variable_assignment_before_offset_from_query(
        source, variable, max_start,
    )
    .and_then(lua_static_identifier_statement_value_from_query);
    if let Some(alias) = alias
        && alias != variable
        && let Some(value) = lua_format_item_table_variable_assignment_from_sources_with_depth(
            source,
            outer_static_source,
            alias,
            max_start,
            depth + 1,
        )
    {
        return Some(value);
    }

    if let Some(outer_static_source) = outer_static_source {
        let alias = lua_static_expression_variable_assignment_before_offset_from_query(
            outer_static_source.source,
            variable,
            outer_static_source.max_start,
        )
        .and_then(lua_static_identifier_statement_value_from_query);
        if let Some(alias) = alias
            && alias != variable
        {
            return lua_format_item_table_variable_assignment_from_sources_with_depth(
                source,
                Some(outer_static_source),
                alias,
                max_start,
                depth + 1,
            );
        }
    }

    None
}

fn lua_static_identifier_statement_value_from_query(value: &str) -> Option<&str> {
    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    lua_static_identifier_value_rest_is_statement_end(rest).then_some(variable)
}

fn lua_format_item_string_variable_assignment_from_sources(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    lua_format_item_string_variable_assignment_from_sources_with_depth(
        source,
        outer_static_source,
        variable,
        max_start,
        0,
    )
}

fn lua_format_item_string_variable_assignment_from_sources_with_depth(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    variable: &str,
    max_start: usize,
    depth: usize,
) -> Option<String> {
    if depth > 8 {
        return None;
    }

    if let Some(value) =
        lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)
    {
        return Some(value.to_owned());
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_string_variable_assignment_before_offset_from_query(
            outer_static_source.source,
            variable,
            outer_static_source.max_start,
        )
    {
        return Some(value.to_owned());
    }

    let alias = lua_static_expression_variable_assignment_before_offset_from_query(
        source, variable, max_start,
    )
    .and_then(lua_static_identifier_statement_value_from_query);
    if let Some(alias) = alias
        && alias != variable
        && let Some(value) = lua_format_item_string_variable_assignment_from_sources_with_depth(
            source,
            outer_static_source,
            alias,
            max_start,
            depth + 1,
        )
    {
        return Some(value);
    }

    if let Some(outer_static_source) = outer_static_source {
        let alias = lua_static_expression_variable_assignment_before_offset_from_query(
            outer_static_source.source,
            variable,
            outer_static_source.max_start,
        )
        .and_then(lua_static_identifier_statement_value_from_query);
        if let Some(alias) = alias
            && alias != variable
        {
            return lua_format_item_string_variable_assignment_from_sources_with_depth(
                source,
                Some(outer_static_source),
                alias,
                max_start,
                depth + 1,
            );
        }
    }

    None
}

fn lua_static_format_items_table_variable_insert_append_value_from_query(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
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
    if let Some(value) =
        lua_format_item_insert_argument_value_from_query(source, outer_static_source, rest, start)
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
        value: lua_format_item_insert_argument_value_from_query(
            source,
            outer_static_source,
            rest,
            start,
        )?,
    })
}

fn lua_format_item_insert_argument_value_from_query(
    source: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    max_start: usize,
) -> Option<String> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some(value.to_owned());
    }

    if let Some(value) = lua_quoted_string_literal_from_query(query)
        .or_else(|| lua_long_bracket_literal_from_query(query))
    {
        let rest = lua_trim_start_comments(query.get(value.len()..)?)?;
        if rest.starts_with(')') {
            return Some(value.to_owned());
        }
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = lua_trim_start_comments(query.get(variable.len()..)?)?;
    if !rest.starts_with(')') {
        return None;
    }
    lua_format_item_table_variable_assignment_from_sources(
        source,
        outer_static_source,
        variable,
        max_start,
    )
    .or_else(|| {
        lua_format_item_string_variable_assignment_from_sources(
            source,
            outer_static_source,
            variable,
            max_start,
        )
    })
}

fn native_format_items_from_lua_format_items_table_query(
    value: &str,
) -> Option<Vec<NativeFormatItem>> {
    native_format_items_from_lua_format_items_table_query_with_static_sources(None, None, value)
}

fn native_format_items_from_lua_format_items_table_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<NativeFormatItem>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut items = Vec::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some(text) = parse_maybe_static_query_text_with_static_sources(
            static_source,
            outer_static_source,
            field,
        ) && text == "ResetAttributes"
        {
            items.push(NativeFormatItem::ResetAttributes);
            continue;
        }
        if let Some(item) = native_format_item_lua_table_from_query_with_static_sources(
            static_source,
            outer_static_source,
            field,
        )
        .or_else(|| {
            native_format_item_from_static_lua_table_variable_with_static_sources(
                static_source,
                outer_static_source,
                field,
            )
        }) {
            items.push(item);
        } else {
            return None;
        }
    }

    Some(items)
}

fn wezterm_format_table_argument_from_query(value: &str) -> Option<&str> {
    let value = value.trim();
    let rest = value
        .strip_prefix("wezterm.format")
        .or_else(|| value.strip_prefix("format"))?
        .trim_start();
    if let Some(argument) = rest.strip_prefix('(') {
        return argument.strip_suffix(')').map(str::trim);
    }
    Some(rest)
}

fn native_format_item_lua_table_from_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeFormatItem> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut item = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_sources(
            static_source,
            outer_static_source,
            key.trim(),
        )?;
        if item.is_some() {
            return None;
        }
        item = Some(match key.as_str() {
            "Text" => NativeFormatItem::Text(parse_maybe_static_query_text_with_static_sources(
                static_source,
                outer_static_source,
                value.trim(),
            )?),
            "Foreground" => NativeFormatItem::Foreground(native_color_spec_to_render_color(
                lua_color_spec_from_query_with_static_sources(
                    static_source,
                    outer_static_source,
                    value.trim(),
                )?,
            )),
            "Background" => NativeFormatItem::Background(native_color_spec_to_render_color(
                lua_color_spec_from_query_with_static_sources(
                    static_source,
                    outer_static_source,
                    value.trim(),
                )?,
            )),
            "Attribute" => NativeFormatItem::Attribute(
                native_format_attribute_lua_table_from_query_with_static_sources(
                    static_source,
                    outer_static_source,
                    value,
                )?,
            ),
            _ => return None,
        });
    }

    item
}

fn native_format_item_from_static_lua_table_variable_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeFormatItem> {
    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    if let Some(static_source) = static_source
        && let Some(item) =
            native_format_item_from_static_lua_table_variable(static_source, variable)
    {
        return item;
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(item) =
            native_format_item_from_static_lua_table_variable(outer_static_source, variable)
    {
        return item;
    }
    None
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn native_format_item_from_static_lua_table_variable(
    static_source: LuaStaticSource<'_>,
    variable: &str,
) -> Option<Option<NativeFormatItem>> {
    let value = lua_static_table_variable_assignment_with_insert_appends_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )?;
    Some(native_format_item_lua_table_from_query_with_static_sources(
        Some(static_source),
        None,
        &value,
    ))
}

fn native_format_attribute_lua_table_from_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeFormatAttribute> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut attribute = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_sources(
            static_source,
            outer_static_source,
            key.trim(),
        )?;
        if attribute.is_some() {
            return None;
        }
        attribute = Some(match key.as_str() {
            "Intensity" => NativeFormatAttribute::Intensity(tab_bar_item_intensity_from_query(
                &parse_maybe_static_query_text_with_static_sources(
                    static_source,
                    outer_static_source,
                    value.trim(),
                )?,
            )?),
            "Italic" => {
                NativeFormatAttribute::Italic(parse_maybe_static_query_bool_with_static_sources(
                    static_source,
                    outer_static_source,
                    value.trim(),
                )?)
            }
            "Underline" => NativeFormatAttribute::Underline(native_format_underline_from_query(
                &parse_maybe_static_query_text_with_static_sources(
                    static_source,
                    outer_static_source,
                    value.trim(),
                )?,
            )?),
            _ => return None,
        });
    }

    attribute
}

fn native_format_underline_from_query(value: &str) -> Option<NativeFormatUnderline> {
    match value {
        "None" => Some(NativeFormatUnderline::None),
        "Single" => Some(NativeFormatUnderline::Single),
        "Double" => Some(NativeFormatUnderline::Double),
        "Curly" => Some(NativeFormatUnderline::Curly),
        "Dotted" => Some(NativeFormatUnderline::Dotted),
        "Dashed" => Some(NativeFormatUnderline::Dashed),
        _ => None,
    }
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_table_field_value_from_query<'a>(
    value: &'a str,
    field_name: &str,
) -> Option<Option<&'a str>> {
    lua_table_field_value_from_query_with_static_source(None, value, field_name)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_table_field_value_from_query_with_static_source<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    value: &'a str,
    field_name: &str,
) -> Option<Option<&'a str>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut found = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != field_name {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(value.trim());
    }

    Some(found)
}

#[derive(Debug, Clone, PartialEq)]
enum NativeStaticLuaColorValue {
    Color(wezterm_color_types::SrgbaTuple),
    Number(f64),
    Integer(u8),
    Bool(bool),
    String(String),
    Tuple(Vec<NativeStaticLuaColorValue>),
}

impl NativeStaticLuaColorValue {
    fn as_color(&self) -> Option<wezterm_color_types::SrgbaTuple> {
        match self {
            Self::Color(color) => Some(*color),
            _ => None,
        }
    }

    fn into_scalar(self) -> Option<Self> {
        (!matches!(self, Self::Tuple(_))).then_some(self)
    }
}

fn terminal_color_from_native_static_lua_color(color: wezterm_color_types::SrgbaTuple) -> Color {
    let (red, green, blue, alpha) = color.to_srgb_u8();
    if alpha == u8::MAX {
        Color::Rgb(red, green, blue)
    } else {
        Color::Rgba(red, green, blue, alpha)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LuaStaticWeztermColorConstructor {
    Parse,
    FromHsla,
}

fn lua_static_wezterm_color_constructor_function_rest_from_query_with_depth<'a>(
    source: &str,
    query: &'a str,
    max_start: usize,
    depth: usize,
) -> Option<(LuaStaticWeztermColorConstructor, &'a str)> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }

    let namespace_rest = lua_static_wezterm_color_namespace_rest_from_query_with_depth(
        source,
        query,
        max_start,
        depth + 1,
    )?;
    let rest = lua_trim_start_comments(namespace_rest)?;
    let (field, rest) = lua_static_string_field_key_from_query(source, max_start, rest)?;
    let constructor = match field.as_str() {
        "parse" => LuaStaticWeztermColorConstructor::Parse,
        "from_hsla" => LuaStaticWeztermColorConstructor::FromHsla,
        _ => return None,
    };
    Some((constructor, rest))
}

fn lua_static_wezterm_color_constructor_value_is_exact_with_depth(
    source: &str,
    value: &str,
    max_start: usize,
    depth: usize,
) -> Option<LuaStaticWeztermColorConstructor> {
    let (constructor, rest) =
        lua_static_wezterm_color_constructor_function_rest_from_query_with_depth(
            source, value, max_start, depth,
        )?;
    lua_static_value_tail_is_value_end(rest).then_some(constructor)
}

fn lua_static_wezterm_color_constructor_call_from_query<'a>(
    static_source: LuaStaticSource<'_>,
    query: &'a str,
) -> Option<(LuaStaticWeztermColorConstructor, &'a str, &'a str)> {
    if !lua_static_wezterm_color_object_api_is_unmodified_before_offset(
        static_source.source,
        static_source.max_start,
    )? {
        return None;
    }
    let query = lua_trim_start_comments(query)?;
    let direct = lua_static_wezterm_color_constructor_function_rest_from_query_with_depth(
        static_source.source,
        query,
        static_source.max_start,
        0,
    );
    let (constructor, rest) = if let Some(direct) = direct {
        direct
    } else {
        let alias = lua_identifier_literal_from_query(query)?;
        let rest = query.get(alias.len()..)?;
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            return None;
        }
        let (binding, binding_start) = lua_static_builtin_scheme_binding_before_offset(
            static_source.source,
            alias,
            static_source.max_start,
        )?;
        let constructor = lua_static_wezterm_color_constructor_value_is_exact_with_depth(
            static_source.source,
            binding,
            binding_start,
            0,
        )?;
        (constructor, rest)
    };

    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let (arguments, tail) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    Some((constructor, arguments, tail))
}

fn lua_static_wezterm_color_object_api_is_unmodified_before_offset(
    source: &str,
    max_start: usize,
) -> Option<bool> {
    for statement_range in lua_top_level_logical_statements_before_offset(source, max_start)? {
        let statement = source.get(statement_range.start..statement_range.end)?;
        let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
        let statement = lua_trim_start_comments(statement)?;
        let assignment_statement = if lua_source_keyword_at(statement, 0, "local") {
            lua_trim_start_comments(statement.get("local".len()..)?)?
        } else {
            statement
        };

        if !lua_source_keyword_at(assignment_statement, 0, "function")
            && let Some((targets, _)) =
                split_lua_static_load_scheme_path_assignment_statement(assignment_statement)
        {
            for target in split_lua_top_level_arguments(targets)? {
                if lua_static_wezterm_color_object_assignment_target_may_modify_api(
                    source,
                    target,
                    statement_range.start,
                )? {
                    return Some(false);
                }
            }
            continue;
        }

        if lua_source_keyword_at(statement, 0, "function") {
            let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
            let function_rest = normalized.get("function".len()..)?.trim_start();
            let Some((target, _)) = function_rest.split_once('(') else {
                continue;
            };
            if lua_static_wezterm_color_object_assignment_target_may_modify_api(
                source,
                &target.replace(':', "."),
                statement_range.start,
            )? {
                return Some(false);
            }
        }
    }
    Some(true)
}

fn lua_static_wezterm_color_object_assignment_target_may_modify_api(
    source: &str,
    target: &str,
    max_start: usize,
) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(target)?;
    let target = normalized.trim();
    if lua_static_load_scheme_path_assignment_target_identifier(target).is_some() {
        return Some(false);
    }

    if let Some(module_rest) =
        lua_static_wezterm_module_namespace_rest_from_query_with_depth(source, target, max_start, 0)
    {
        let rest = lua_trim_start_comments(module_rest)?;
        let Some((field, _)) = lua_static_string_field_key_from_query(source, max_start, rest)
        else {
            return Some(rest.starts_with('.') || rest.starts_with('['));
        };
        return Some(field == "color");
    }

    if lua_static_wezterm_color_namespace_rest_from_query_with_depth(source, target, max_start, 0)
        .is_some()
    {
        return Some(true);
    }

    Some(false)
}

fn lua_static_wezterm_color_value_from_query(
    static_source: LuaStaticSource<'_>,
    query: &str,
) -> Option<NativeStaticLuaColorValue> {
    lua_static_wezterm_color_value_from_query_with_depth(static_source, query, 0)
}

fn lua_static_wezterm_color_value_from_query_with_depth(
    static_source: LuaStaticSource<'_>,
    query: &str,
    depth: usize,
) -> Option<NativeStaticLuaColorValue> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }
    let query = lua_trim_start_comments(query)?;
    if let Some(argument) = lua_tostring_argument_from_query(query) {
        let color = lua_static_wezterm_color_value_from_query_with_depth(
            static_source,
            argument,
            depth + 1,
        )?
        .into_scalar()?
        .as_color()?;
        return Some(NativeStaticLuaColorValue::String(color.to_string()));
    }
    if let Some((left, right, equal)) = lua_static_wezterm_color_equality_operands(query) {
        let left =
            lua_static_wezterm_color_value_from_query_with_depth(static_source, left, depth + 1)?
                .into_scalar()?
                .as_color()?;
        let right =
            lua_static_wezterm_color_value_from_query_with_depth(static_source, right, depth + 1)?
                .into_scalar()?
                .as_color()?;
        return Some(NativeStaticLuaColorValue::Bool((left == right) == equal));
    }
    let (mut value, mut tail) = if let Some((constructor, arguments, tail)) =
        lua_static_wezterm_color_constructor_call_from_query(static_source, query)
    {
        let arguments = split_lua_top_level_arguments(arguments)?;
        let color = match constructor {
            LuaStaticWeztermColorConstructor::Parse => {
                let [argument] = arguments.as_slice() else {
                    return None;
                };
                let value = parse_maybe_static_query_text(Some(static_source), argument)?;
                value.parse::<wezterm_color_types::SrgbaTuple>().ok()?
            }
            LuaStaticWeztermColorConstructor::FromHsla => {
                let [hue, saturation, lightness, alpha] = arguments.as_slice() else {
                    return None;
                };
                let values = [hue, saturation, lightness, alpha]
                    .map(|value| lua_static_wezterm_color_method_number(static_source, value))
                    .into_iter()
                    .collect::<Option<Vec<_>>>()?;
                if values.iter().any(|value| !value.is_finite()) {
                    return None;
                }
                wezterm_color_types::SrgbaTuple::from_hsla(
                    values[0], values[1], values[2], values[3],
                )
            }
        };
        (NativeStaticLuaColorValue::Color(color), tail)
    } else {
        let variable = lua_identifier_literal_from_query(query)?;
        let tail = query.get(variable.len()..)?;
        if tail.chars().next().is_some_and(is_lua_identifier_character) {
            return None;
        }
        let value = lua_static_color_value_binding_before_offset(
            static_source.source,
            variable,
            static_source.max_start,
            depth + 1,
        )?;
        (value, tail)
    };

    loop {
        if lua_static_value_tail_is_value_end(tail) {
            return Some(value);
        }
        let rest = lua_trim_start_comments(tail)?.strip_prefix(':')?;
        let rest = lua_trim_start_comments(rest)?;
        let method = lua_identifier_literal_from_query(rest)?;
        let rest = lua_trim_start_comments(rest.get(method.len()..)?)?.strip_prefix('(')?;
        let (arguments, next_tail) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        let arguments = if lua_static_load_scheme_path_query_without_comments(arguments)?
            .trim()
            .is_empty()
        {
            Vec::new()
        } else {
            split_lua_top_level_arguments(arguments)?
        };
        let receiver = value.as_color()?;
        value = lua_static_wezterm_color_method_result(
            static_source,
            receiver,
            method,
            &arguments,
            depth + 1,
        )?;
        tail = next_tail;
    }
}

fn lua_static_wezterm_color_equality_operands(query: &str) -> Option<(&str, &str, bool)> {
    let (left, right, equal) = if let Some((left, right)) = query.split_once("~=") {
        (!right.contains("~=") && !left.contains("==") && !right.contains("=="))
            .then_some((left, right, false))?
    } else {
        let (left, right) = query.split_once("==")?;
        (!right.contains("==") && !left.contains("~=") && !right.contains("~="))
            .then_some((left, right, true))?
    };
    let left = lua_trim_start_comments(left)?;
    let left = lua_trim_end_comments(left)?.trim();
    let right = lua_trim_start_comments(right)?;
    let right = lua_trim_end_comments(right)?.trim();
    (!left.is_empty() && !right.is_empty()).then_some((left, right, equal))
}

fn lua_static_wezterm_color_method_number(
    static_source: LuaStaticSource<'_>,
    query: &str,
) -> Option<f64> {
    lua_static_wezterm_color_method_number_with_depth(static_source, query, 0)
}

fn lua_static_wezterm_color_method_number_with_depth(
    static_source: LuaStaticSource<'_>,
    query: &str,
    depth: usize,
) -> Option<f64> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }
    let query = lua_trim_start_comments(query)?;
    let query = lua_trim_end_comments(query)?.trim();
    let number: f64 = if let Some(literal) = lua_signed_number_literal_from_query(query)
        && lua_static_value_tail_is_value_end(query.get(literal.len()..)?)
    {
        literal.parse().ok()?
    } else {
        let variable = lua_identifier_literal_from_query(query)?;
        let rest = query.get(variable.len()..)?;
        if !lua_static_value_tail_is_value_end(rest) {
            return None;
        }
        let (binding, binding_start) = lua_static_builtin_scheme_binding_before_offset(
            static_source.source,
            variable,
            static_source.max_start,
        )?;
        return lua_static_wezterm_color_method_number_with_depth(
            LuaStaticSource {
                source: static_source.source,
                max_start: binding_start,
            },
            binding,
            depth + 1,
        );
    };
    number.is_finite().then_some(number)
}

fn lua_static_wezterm_color_method_color(
    static_source: LuaStaticSource<'_>,
    query: &str,
    depth: usize,
) -> Option<wezterm_color_types::SrgbaTuple> {
    let query = lua_trim_start_comments(query)?;
    let query = lua_trim_end_comments(query)?.trim();
    lua_static_wezterm_color_value_from_query_with_depth(static_source, query, depth + 1)?
        .into_scalar()?
        .as_color()
}

fn native_static_lua_number_tuple(values: (f64, f64, f64, f64)) -> NativeStaticLuaColorValue {
    NativeStaticLuaColorValue::Tuple(vec![
        NativeStaticLuaColorValue::Number(values.0),
        NativeStaticLuaColorValue::Number(values.1),
        NativeStaticLuaColorValue::Number(values.2),
        NativeStaticLuaColorValue::Number(values.3),
    ])
}

fn lua_static_wezterm_color_method_result(
    static_source: LuaStaticSource<'_>,
    receiver: wezterm_color_types::SrgbaTuple,
    method: &str,
    arguments: &[&str],
    depth: usize,
) -> Option<NativeStaticLuaColorValue> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }
    let value = match (method, arguments) {
        ("complement", []) => NativeStaticLuaColorValue::Color(receiver.complement()),
        ("complement_ryb", []) => NativeStaticLuaColorValue::Color(receiver.complement_ryb()),
        ("saturate", [factor]) => NativeStaticLuaColorValue::Color(receiver.saturate(
            lua_static_wezterm_color_method_number(static_source, factor)?,
        )),
        ("desaturate", [factor]) => NativeStaticLuaColorValue::Color(receiver.saturate(
            -lua_static_wezterm_color_method_number(static_source, factor)?,
        )),
        ("saturate_fixed", [amount]) => NativeStaticLuaColorValue::Color(receiver.saturate_fixed(
            lua_static_wezterm_color_method_number(static_source, amount)?,
        )),
        ("desaturate_fixed", [amount]) => {
            NativeStaticLuaColorValue::Color(receiver.saturate_fixed(
                -lua_static_wezterm_color_method_number(static_source, amount)?,
            ))
        }
        ("lighten", [factor]) => NativeStaticLuaColorValue::Color(receiver.lighten(
            lua_static_wezterm_color_method_number(static_source, factor)?,
        )),
        ("darken", [factor]) => NativeStaticLuaColorValue::Color(receiver.lighten(
            -lua_static_wezterm_color_method_number(static_source, factor)?,
        )),
        ("lighten_fixed", [amount]) => NativeStaticLuaColorValue::Color(receiver.lighten_fixed(
            lua_static_wezterm_color_method_number(static_source, amount)?,
        )),
        ("darken_fixed", [amount]) => NativeStaticLuaColorValue::Color(receiver.lighten_fixed(
            -lua_static_wezterm_color_method_number(static_source, amount)?,
        )),
        ("adjust_hue_fixed", [amount]) => {
            NativeStaticLuaColorValue::Color(receiver.adjust_hue_fixed(
                lua_static_wezterm_color_method_number(static_source, amount)?,
            ))
        }
        ("adjust_hue_fixed_ryb", [amount]) => {
            NativeStaticLuaColorValue::Color(receiver.adjust_hue_fixed_ryb(
                lua_static_wezterm_color_method_number(static_source, amount)?,
            ))
        }
        ("triad", []) => {
            let (first, second) = receiver.triad();
            NativeStaticLuaColorValue::Tuple(vec![
                NativeStaticLuaColorValue::Color(first),
                NativeStaticLuaColorValue::Color(second),
            ])
        }
        ("square", []) => {
            let (first, second, third) = receiver.square();
            NativeStaticLuaColorValue::Tuple(vec![
                NativeStaticLuaColorValue::Color(first),
                NativeStaticLuaColorValue::Color(second),
                NativeStaticLuaColorValue::Color(third),
            ])
        }
        ("srgba_u8", []) => {
            let (red, green, blue, alpha) = receiver.to_srgb_u8();
            NativeStaticLuaColorValue::Tuple(vec![
                NativeStaticLuaColorValue::Integer(red),
                NativeStaticLuaColorValue::Integer(green),
                NativeStaticLuaColorValue::Integer(blue),
                NativeStaticLuaColorValue::Integer(alpha),
            ])
        }
        ("linear_rgba", []) => {
            let linear = receiver.to_linear();
            NativeStaticLuaColorValue::Tuple(vec![
                NativeStaticLuaColorValue::Number(f64::from(linear.0)),
                NativeStaticLuaColorValue::Number(f64::from(linear.1)),
                NativeStaticLuaColorValue::Number(f64::from(linear.2)),
                NativeStaticLuaColorValue::Number(f64::from(linear.3)),
            ])
        }
        ("hsla", []) => native_static_lua_number_tuple(receiver.to_hsla()),
        ("laba", []) => native_static_lua_number_tuple(receiver.to_laba()),
        ("contrast_ratio", [other]) => NativeStaticLuaColorValue::Number(receiver.contrast_ratio(
            &lua_static_wezterm_color_method_color(static_source, other, depth + 1)?,
        )),
        ("delta_e", [other]) => NativeStaticLuaColorValue::Number(f64::from(receiver.delta_e(
            &lua_static_wezterm_color_method_color(static_source, other, depth + 1)?,
        ))),
        _ => return None,
    };
    Some(value)
}

fn lua_static_color_value_binding_before_offset(
    source: &str,
    variable: &str,
    max_start: usize,
    depth: usize,
) -> Option<NativeStaticLuaColorValue> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        let Some((targets, value)) = rest.split_once('=') else {
            continue;
        };
        if targets.contains('\n') || targets.contains('\r') || targets.contains(';') {
            continue;
        }
        let targets = split_lua_top_level_arguments(targets)?;
        if targets.iter().any(|target| {
            lua_static_color_assignment_target_may_modify_variable(target, variable).unwrap_or(true)
        }) {
            selected = None;
            continue;
        }
        let Some(target_index) = targets.iter().position(|target| {
            lua_static_load_scheme_path_assignment_target_identifier(target).as_deref()
                == Some(variable)
        }) else {
            continue;
        };
        let Some(value) = lua_top_level_statement_value_from_query(value) else {
            selected = None;
            continue;
        };
        let alias_variable = lua_identifier_literal_from_query(value).filter(|alias| {
            value
                .get(alias.len()..)
                .is_some_and(lua_static_value_tail_is_value_end)
        });
        let evaluated = lua_static_wezterm_color_value_from_query_with_depth(
            LuaStaticSource {
                source,
                max_start: start,
            },
            value,
            depth + 1,
        );
        if let Some(alias_variable) = alias_variable
            && lua_static_color_variable_is_modified_between_offsets(
                source,
                alias_variable,
                start,
                max_start,
            )?
        {
            selected = None;
            continue;
        }
        selected = match evaluated {
            Some(NativeStaticLuaColorValue::Tuple(values)) => values.get(target_index).cloned(),
            Some(value) if targets.len() == 1 && target_index == 0 => value.into_scalar(),
            Some(_) | None => None,
        };
    }

    selected
}

fn lua_static_color_variable_is_modified_between_offsets(
    source: &str,
    variable: &str,
    min_start: usize,
    max_start: usize,
) -> Option<bool> {
    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if start <= min_start {
            continue;
        }
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        let Some((targets, _)) = rest.split_once('=') else {
            continue;
        };
        if targets.contains('\n') || targets.contains('\r') || targets.contains(';') {
            continue;
        }
        for target in split_lua_top_level_arguments(targets)? {
            if lua_static_load_scheme_path_assignment_target_identifier(target).as_deref()
                == Some(variable)
                || lua_static_color_assignment_target_may_modify_variable(target, variable)?
            {
                return Some(true);
            }
        }
    }
    Some(false)
}

fn lua_static_color_assignment_target_may_modify_variable(
    target: &str,
    variable: &str,
) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(target)?;
    let target = normalized.trim();
    let Some(rest) = target.strip_prefix(variable) else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?;
    Some(rest.starts_with('.') || rest.starts_with('['))
}

fn lua_static_color_number_from_query(
    static_source: LuaStaticSource<'_>,
    query: &str,
) -> Option<f64> {
    match lua_static_wezterm_color_value_from_query(static_source, query)?.into_scalar()? {
        NativeStaticLuaColorValue::Number(value) => Some(value),
        NativeStaticLuaColorValue::Integer(value) => Some(f64::from(value)),
        _ => None,
    }
}

fn lua_static_color_bool_from_query(
    static_source: LuaStaticSource<'_>,
    query: &str,
) -> Option<bool> {
    match lua_static_wezterm_color_value_from_query(static_source, query)?.into_scalar()? {
        NativeStaticLuaColorValue::Bool(value) => Some(value),
        _ => None,
    }
}

fn lua_static_color_string_from_query(
    static_source: LuaStaticSource<'_>,
    query: &str,
) -> Option<String> {
    match lua_static_wezterm_color_value_from_query(static_source, query)?.into_scalar()? {
        NativeStaticLuaColorValue::String(value) => Some(value),
        _ => None,
    }
}

fn lua_color_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> String {
    lua_static_wezterm_color_parse_alias_query_from_query(static_source, value)
        .unwrap_or_else(|| value.to_owned())
}

fn lua_color_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> String {
    lua_static_wezterm_color_parse_alias_query_from_query(static_source, value)
        .or_else(|| {
            lua_static_wezterm_color_parse_alias_query_from_query(outer_static_source, value)
        })
        .unwrap_or_else(|| value.to_owned())
}

fn lua_color_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Color> {
    if let Some(color) = static_source
        .and_then(|source| lua_static_wezterm_color_value_from_query(source, value))
        .and_then(NativeStaticLuaColorValue::into_scalar)
        .and_then(|value| value.as_color())
    {
        return Some(terminal_color_from_native_static_lua_color(color));
    }
    let value = lua_color_query_with_static_source(static_source, value);
    lua_color_from_query(&value)
}

fn lua_opaque_color_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Color> {
    if let Some(color) = static_source
        .and_then(|source| lua_static_wezterm_color_value_from_query(source, value))
        .and_then(NativeStaticLuaColorValue::into_scalar)
        .and_then(|value| value.as_color())
    {
        return Some(opaque_color(terminal_color_from_native_static_lua_color(
            color,
        )));
    }
    let value = lua_color_query_with_static_source(static_source, value);
    lua_opaque_color_from_query(&value)
}

fn lua_opaque_color_from_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Color> {
    if let Some(color) = static_source
        .and_then(|source| lua_static_wezterm_color_value_from_query(source, value))
        .or_else(|| {
            outer_static_source
                .and_then(|source| lua_static_wezterm_color_value_from_query(source, value))
        })
        .and_then(NativeStaticLuaColorValue::into_scalar)
        .and_then(|value| value.as_color())
    {
        return Some(opaque_color(terminal_color_from_native_static_lua_color(
            color,
        )));
    }
    let value = lua_color_query_with_static_sources(static_source, outer_static_source, value);
    lua_opaque_color_from_query(&value)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_lua_table_field_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    field_name: &str,
) -> Option<Option<Color>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut color = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != field_name {
            continue;
        }
        if color.is_some() {
            return None;
        }
        let value = parse_maybe_quoted_query_text(value.trim())?;
        color = Some(lua_opaque_color_from_query_with_static_source(
            static_source,
            &value,
        )?);
    }

    Some(color)
}

fn lua_static_wezterm_color_parse_alias_query_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<String> {
    let static_source = static_source?;
    let query = lua_trim_start_comments(query)?;
    let alias = lua_identifier_literal_from_query(query)?;
    if !lua_static_wezterm_color_parse_alias_before_offset(
        static_source.source,
        alias,
        static_source.max_start,
    )? {
        return None;
    }
    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    if !matches!(rest.chars().next()?, '(') {
        return None;
    }

    Some(format!("wezterm.color.parse{rest}"))
}

fn lua_static_wezterm_color_parse_alias_before_offset(
    source: &str,
    alias: &str,
    max_start: usize,
) -> Option<bool> {
    let mut selected = false;

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
        selected = lua_static_wezterm_color_parse_alias_value_from_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_color_parse_alias_value_from_query(
    source: &str,
    max_start: usize,
    value: &str,
) -> bool {
    let Some(value) = lua_trim_start_comments(value) else {
        return false;
    };
    let rest = if let Some(rest) = lua_static_wezterm_require_receiver_rest_from_query(value) {
        rest
    } else if let Some(rest) = lua_static_wezterm_receiver_rest_from_query(source, max_start, value)
    {
        rest
    } else {
        return false;
    };
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let static_source = Some(LuaStaticSource { source, max_start });
    let Some((field, rest)) =
        lua_table_map_field_key_from_query_with_static_source(static_source, rest)
    else {
        return false;
    };
    if field != "color" {
        return false;
    }
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some((field, rest)) =
        lua_table_map_field_key_from_query_with_static_source(static_source, rest)
    else {
        return false;
    };
    field == "parse" && lua_static_identifier_value_rest_is_statement_end(rest)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_spec_lua_table_field_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    field_name: &str,
) -> Option<Option<NativeColorSpec>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut color = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != field_name {
            continue;
        }
        if color.is_some() {
            return None;
        }
        color = Some(lua_color_spec_from_query_with_static_source(
            static_source,
            value.trim(),
        )?);
    }

    Some(color)
}

fn lua_color_spec_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeColorSpec> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut color = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if color.is_some() {
            return None;
        }
        let value = parse_maybe_static_query_text(static_source, value.trim())?;
        color = Some(match key.as_str() {
            "Color" => NativeColorSpec::Color(lua_opaque_color_from_query_with_static_source(
                static_source,
                &value,
            )?),
            "AnsiColor" => NativeColorSpec::AnsiColor(NativeAnsiColor::parse(&value)?),
            _ => return None,
        });
    }

    color
}

fn lua_color_spec_from_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeColorSpec> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut color = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_sources(
            static_source,
            outer_static_source,
            key.trim(),
        )?;
        if color.is_some() {
            return None;
        }
        let value = parse_maybe_static_query_text_with_static_sources(
            static_source,
            outer_static_source,
            value.trim(),
        )?;
        color = Some(match key.as_str() {
            "Color" => NativeColorSpec::Color(lua_opaque_color_from_query_with_static_sources(
                static_source,
                outer_static_source,
                &value,
            )?),
            "AnsiColor" => NativeColorSpec::AnsiColor(NativeAnsiColor::parse(&value)?),
            _ => return None,
        });
    }

    color
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn selection_fg_lua_table_field_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Option<Option<Color>>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut color = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != "selection_fg" {
            continue;
        }
        if color.is_some() {
            return None;
        }
        let value = parse_maybe_quoted_query_text(value.trim())?;
        let value = lua_color_query_with_static_source(static_source, &value);
        color = Some(if value.eq_ignore_ascii_case("none") {
            None
        } else {
            lua_selection_foreground_color_from_query(&value)?
        });
    }

    Some(color)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn selection_bg_lua_table_field_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Option<Color>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut color = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != "selection_bg" {
            continue;
        }
        if color.is_some() {
            return None;
        }
        let value = parse_maybe_quoted_query_text(value.trim())?;
        color = Some(lua_color_from_query_with_static_source(
            static_source,
            &value,
        )?);
    }

    Some(color)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn color_array_lua_table_field_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    field_name: &str,
) -> Option<Option<[Color; 8]>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut colors = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != field_name {
            continue;
        }
        if colors.is_some() {
            return None;
        }
        let values =
            split_lua_table_color_expression_array_with_static_source(static_source, value.trim())?;
        let parsed = values
            .iter()
            .map(|value| lua_opaque_color_from_query_with_static_source(static_source, value))
            .collect::<Option<Vec<_>>>()?;
        colors = Some(<[Color; 8]>::try_from(parsed).ok()?);
    }

    Some(colors)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn indexed_palette_lua_table_field_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Option<[Option<Color>; 256]>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut indexed_palette = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key != "indexed" {
            continue;
        }
        if indexed_palette.is_some() {
            return None;
        }

        let indexed_table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        let mut palette = [None; 256];
        for entry in split_lua_table_top_level_fields(indexed_table)? {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let (index, color) = split_lua_table_assignment_from_field(entry)?;
            let index = split_lua_table_array_index_from_query(index.trim())?;
            if !(16..=255).contains(&index) || palette[index].is_some() {
                return None;
            }
            let color = parse_maybe_quoted_query_text(color.trim())?;
            palette[index] = Some(lua_opaque_color_from_query_with_static_source(
                static_source,
                &color,
            )?);
        }
        indexed_palette = Some(palette);
    }

    Some(indexed_palette)
}

fn lua_hex_color_from_query(value: &str) -> Option<Color> {
    let hex = value.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(red, green, blue))
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_named_color_from_query(value: &str) -> Option<Color> {
    let color = match value.trim().to_ascii_lowercase().as_str() {
        "aliceblue" => Color::Rgb(240, 248, 255),
        "antiquewhite" => Color::Rgb(250, 235, 215),
        "aqua" | "cyan" => Color::Rgb(0, 255, 255),
        "aquamarine" => Color::Rgb(127, 255, 212),
        "azure" => Color::Rgb(240, 255, 255),
        "beige" => Color::Rgb(245, 245, 220),
        "bisque" => Color::Rgb(255, 228, 196),
        "black" => Color::Rgb(0, 0, 0),
        "blanchedalmond" => Color::Rgb(255, 235, 205),
        "blue" => Color::Rgb(0, 0, 255),
        "blueviolet" => Color::Rgb(138, 43, 226),
        "brown" => Color::Rgb(165, 42, 42),
        "burlywood" => Color::Rgb(222, 184, 135),
        "cadetblue" => Color::Rgb(95, 158, 160),
        "chartreuse" => Color::Rgb(127, 255, 0),
        "chocolate" => Color::Rgb(210, 105, 30),
        "coral" => Color::Rgb(255, 127, 80),
        "cornflowerblue" => Color::Rgb(100, 149, 237),
        "cornsilk" => Color::Rgb(255, 248, 220),
        "crimson" => Color::Rgb(220, 20, 60),
        "darkblue" => Color::Rgb(0, 0, 139),
        "darkcyan" => Color::Rgb(0, 139, 139),
        "darkgoldenrod" => Color::Rgb(184, 134, 11),
        "darkgray" | "darkgrey" => Color::Rgb(169, 169, 169),
        "darkgreen" => Color::Rgb(0, 100, 0),
        "darkkhaki" => Color::Rgb(189, 183, 107),
        "darkmagenta" => Color::Rgb(139, 0, 139),
        "darkolivegreen" => Color::Rgb(85, 107, 47),
        "darkorange" => Color::Rgb(255, 140, 0),
        "darkorchid" => Color::Rgb(153, 50, 204),
        "darkred" => Color::Rgb(139, 0, 0),
        "darksalmon" => Color::Rgb(233, 150, 122),
        "darkseagreen" => Color::Rgb(143, 188, 143),
        "darkslateblue" => Color::Rgb(72, 61, 139),
        "darkslategray" | "darkslategrey" => Color::Rgb(47, 79, 79),
        "darkturquoise" => Color::Rgb(0, 206, 209),
        "darkviolet" => Color::Rgb(148, 0, 211),
        "deeppink" => Color::Rgb(255, 20, 147),
        "deepskyblue" => Color::Rgb(0, 191, 255),
        "dimgray" | "dimgrey" => Color::Rgb(105, 105, 105),
        "dodgerblue" => Color::Rgb(30, 144, 255),
        "firebrick" => Color::Rgb(178, 34, 34),
        "floralwhite" => Color::Rgb(255, 250, 240),
        "forestgreen" => Color::Rgb(34, 139, 34),
        "fuchsia" | "magenta" => Color::Rgb(255, 0, 255),
        "gainsboro" => Color::Rgb(220, 220, 220),
        "ghostwhite" => Color::Rgb(248, 248, 255),
        "gold" => Color::Rgb(255, 215, 0),
        "goldenrod" => Color::Rgb(218, 165, 32),
        "gray" | "grey" => Color::Rgb(128, 128, 128),
        "green" => Color::Rgb(0, 128, 0),
        "greenyellow" => Color::Rgb(173, 255, 47),
        "honeydew" => Color::Rgb(240, 255, 240),
        "hotpink" => Color::Rgb(255, 105, 180),
        "indianred" => Color::Rgb(205, 92, 92),
        "indigo" => Color::Rgb(75, 0, 130),
        "ivory" => Color::Rgb(255, 255, 240),
        "khaki" => Color::Rgb(240, 230, 140),
        "lavender" => Color::Rgb(230, 230, 250),
        "lavenderblush" => Color::Rgb(255, 240, 245),
        "lawngreen" => Color::Rgb(124, 252, 0),
        "lemonchiffon" => Color::Rgb(255, 250, 205),
        "lightblue" => Color::Rgb(173, 216, 230),
        "lightcoral" => Color::Rgb(240, 128, 128),
        "lightcyan" => Color::Rgb(224, 255, 255),
        "lightgoldenrodyellow" => Color::Rgb(250, 250, 210),
        "lightgray" | "lightgrey" => Color::Rgb(211, 211, 211),
        "lightgreen" => Color::Rgb(144, 238, 144),
        "lightpink" => Color::Rgb(255, 182, 193),
        "lightsalmon" => Color::Rgb(255, 160, 122),
        "lightseagreen" => Color::Rgb(32, 178, 170),
        "lightskyblue" => Color::Rgb(135, 206, 250),
        "lightslategray" | "lightslategrey" => Color::Rgb(119, 136, 153),
        "lightsteelblue" => Color::Rgb(176, 196, 222),
        "lightyellow" => Color::Rgb(255, 255, 224),
        "lime" => Color::Rgb(0, 255, 0),
        "limegreen" => Color::Rgb(50, 205, 50),
        "linen" => Color::Rgb(250, 240, 230),
        "maroon" => Color::Rgb(128, 0, 0),
        "mediumaquamarine" => Color::Rgb(102, 205, 170),
        "mediumblue" => Color::Rgb(0, 0, 205),
        "mediumorchid" => Color::Rgb(186, 85, 211),
        "mediumpurple" => Color::Rgb(147, 112, 219),
        "mediumseagreen" => Color::Rgb(60, 179, 113),
        "mediumslateblue" => Color::Rgb(123, 104, 238),
        "mediumspringgreen" => Color::Rgb(0, 250, 154),
        "mediumturquoise" => Color::Rgb(72, 209, 204),
        "mediumvioletred" => Color::Rgb(199, 21, 133),
        "midnightblue" => Color::Rgb(25, 25, 112),
        "mintcream" => Color::Rgb(245, 255, 250),
        "mistyrose" => Color::Rgb(255, 228, 225),
        "moccasin" => Color::Rgb(255, 228, 181),
        "navajowhite" => Color::Rgb(255, 222, 173),
        "navy" => Color::Rgb(0, 0, 128),
        "oldlace" => Color::Rgb(253, 245, 230),
        "olive" => Color::Rgb(128, 128, 0),
        "olivedrab" => Color::Rgb(107, 142, 35),
        "orange" => Color::Rgb(255, 165, 0),
        "orangered" => Color::Rgb(255, 69, 0),
        "orchid" => Color::Rgb(218, 112, 214),
        "palegoldenrod" => Color::Rgb(238, 232, 170),
        "palegreen" => Color::Rgb(152, 251, 152),
        "paleturquoise" => Color::Rgb(175, 238, 238),
        "palevioletred" => Color::Rgb(219, 112, 147),
        "papayawhip" => Color::Rgb(255, 239, 213),
        "peachpuff" => Color::Rgb(255, 218, 185),
        "peru" => Color::Rgb(205, 133, 63),
        "pink" => Color::Rgb(255, 192, 203),
        "plum" => Color::Rgb(221, 160, 221),
        "powderblue" => Color::Rgb(176, 224, 230),
        "purple" => Color::Rgb(128, 0, 128),
        "rebeccapurple" => Color::Rgb(102, 51, 153),
        "red" => Color::Rgb(255, 0, 0),
        "rosybrown" => Color::Rgb(188, 143, 143),
        "royalblue" => Color::Rgb(65, 105, 225),
        "saddlebrown" => Color::Rgb(139, 69, 19),
        "salmon" => Color::Rgb(250, 128, 114),
        "sandybrown" => Color::Rgb(244, 164, 96),
        "seagreen" => Color::Rgb(46, 139, 87),
        "seashell" => Color::Rgb(255, 245, 238),
        "sienna" => Color::Rgb(160, 82, 45),
        "silver" => Color::Rgb(192, 192, 192),
        "skyblue" => Color::Rgb(135, 206, 235),
        "slateblue" => Color::Rgb(106, 90, 205),
        "slategray" | "slategrey" => Color::Rgb(112, 128, 144),
        "snow" => Color::Rgb(255, 250, 250),
        "springgreen" => Color::Rgb(0, 255, 127),
        "steelblue" => Color::Rgb(70, 130, 180),
        "tan" => Color::Rgb(210, 180, 140),
        "teal" => Color::Rgb(0, 128, 128),
        "thistle" => Color::Rgb(216, 191, 216),
        "tomato" => Color::Rgb(255, 99, 71),
        "transparent" => Color::Rgba(0, 0, 0, 0),
        "turquoise" => Color::Rgb(64, 224, 208),
        "violet" => Color::Rgb(238, 130, 238),
        "wheat" => Color::Rgb(245, 222, 179),
        "white" => Color::Rgb(255, 255, 255),
        "whitesmoke" => Color::Rgb(245, 245, 245),
        "yellow" => Color::Rgb(255, 255, 0),
        "yellowgreen" => Color::Rgb(154, 205, 50),
        _ => return None,
    };
    Some(color)
}

fn lua_color_from_query(value: &str) -> Option<Color> {
    lua_wezterm_color_parse_from_query(value)
        .or_else(|| lua_hex_color_from_query(value))
        .or_else(|| lua_named_color_from_query(value))
        .or_else(|| lua_rgb_color_from_query(value))
        .or_else(|| lua_hsl_color_from_query(value))
        .or_else(|| lua_hwb_color_from_query(value))
        .or_else(|| lua_hsv_color_from_query(value))
        .or_else(|| lua_rgba_color_from_query(value))
}

fn lua_opaque_color_from_query(value: &str) -> Option<Color> {
    Some(opaque_color(lua_color_from_query(value)?))
}

fn lua_wezterm_color_parse_from_query(value: &str) -> Option<Color> {
    let value = strip_lua_function_call_from_query(value, "wezterm.color.parse")?;
    let value = parse_maybe_quoted_query_text(value)?;
    lua_color_from_query(&value)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_selection_foreground_color_from_query(value: &str) -> Option<Option<Color>> {
    let color = lua_color_from_query(value)?;
    match color {
        Color::Rgba(_, _, _, 0) => Some(None),
        color => Some(Some(color)),
    }
}

fn opaque_color(color: Color) -> Color {
    match color {
        Color::Rgba(red, green, blue, _) => Color::Rgb(red, green, blue),
        color => color,
    }
}

fn lua_rgb_color_from_query(value: &str) -> Option<Color> {
    let components = lua_color_function_body(value, "rgb")?;
    let (channels, alpha) = split_lua_css_rgb_channels_and_alpha(components);
    let [red, green, blue] = <[&str; 3]>::try_from(channels).ok()?;
    let red = lua_css_rgb_channel_from_query(red)?;
    let green = lua_css_rgb_channel_from_query(green)?;
    let blue = lua_css_rgb_channel_from_query(blue)?;
    alpha.map_or_else(
        || Some(Color::Rgb(red, green, blue)),
        |alpha| {
            Some(Color::Rgba(
                red,
                green,
                blue,
                lua_css_alpha_from_query(alpha)?,
            ))
        },
    )
}

fn lua_rgba_color_from_query(value: &str) -> Option<Color> {
    let components = lua_color_function_body(value, "rgba")?;
    let (mut components, alpha) = split_lua_css_rgb_channels_and_alpha(components);
    let alpha = alpha.or_else(|| {
        if components.len() == 4 {
            components.pop()
        } else {
            None
        }
    })?;
    let [red, green, blue] = <[&str; 3]>::try_from(components).ok()?;
    let red = lua_css_rgb_channel_from_query(red)?;
    let green = lua_css_rgb_channel_from_query(green)?;
    let blue = lua_css_rgb_channel_from_query(blue)?;
    let alpha = lua_css_alpha_from_query(alpha)?;
    Some(Color::Rgba(red, green, blue, alpha))
}

fn lua_hsl_color_from_query(value: &str) -> Option<Color> {
    let (channels, alpha) = if let Some(components) = value.trim().strip_prefix("hsl:") {
        (components.split_whitespace().collect::<Vec<_>>(), None)
    } else if let Some(components) = lua_color_function_body(value, "hsl") {
        split_lua_css_rgb_channels_and_alpha(components)
    } else {
        let components = lua_color_function_body(value, "hsla")?;
        let (mut channels, alpha) = split_lua_css_rgb_channels_and_alpha(components);
        let alpha = alpha.or_else(|| {
            if channels.len() == 4 {
                channels.pop()
            } else {
                None
            }
        });
        (channels, alpha)
    };
    let [hue, saturation, lightness] = <[&str; 3]>::try_from(channels).ok()?;
    let [red, green, blue] = hsl_to_rgb(
        lua_css_hue_degrees_from_query(hue)?,
        lua_css_percentage_from_query(saturation)?,
        lua_css_percentage_from_query(lightness)?,
    );
    alpha.map_or_else(
        || Some(Color::Rgb(red, green, blue)),
        |alpha| {
            Some(Color::Rgba(
                red,
                green,
                blue,
                lua_css_alpha_from_query(alpha)?,
            ))
        },
    )
}

fn lua_hsv_color_from_query(value: &str) -> Option<Color> {
    let components = lua_color_function_body(value, "hsv")?;
    let (channels, alpha) = split_lua_css_rgb_channels_and_alpha(components);
    let [hue, saturation, value] = <[&str; 3]>::try_from(channels).ok()?;
    let [red, green, blue] = hsv_to_rgb(
        lua_css_hue_degrees_from_query(hue)?,
        lua_css_percentage_from_query(saturation)?,
        lua_css_percentage_from_query(value)?,
    );
    alpha.map_or_else(
        || Some(Color::Rgb(red, green, blue)),
        |alpha| {
            Some(Color::Rgba(
                red,
                green,
                blue,
                lua_css_alpha_from_query(alpha)?,
            ))
        },
    )
}

fn lua_hwb_color_from_query(value: &str) -> Option<Color> {
    let components = lua_color_function_body(value, "hwb")?;
    let (channels, alpha) = split_lua_css_rgb_channels_and_alpha(components);
    let [hue, whiteness, blackness] = <[&str; 3]>::try_from(channels).ok()?;
    let [red, green, blue] = hwb_to_rgb(
        lua_css_hue_degrees_from_query(hue)?,
        lua_css_percentage_from_query(whiteness)?,
        lua_css_percentage_from_query(blackness)?,
    );
    alpha.map_or_else(
        || Some(Color::Rgb(red, green, blue)),
        |alpha| {
            Some(Color::Rgba(
                red,
                green,
                blue,
                lua_css_alpha_from_query(alpha)?,
            ))
        },
    )
}

fn lua_color_function_body<'a>(value: &'a str, function: &str) -> Option<&'a str> {
    let function = format!("{function}(");
    value
        .trim()
        .strip_prefix(function.as_str())?
        .strip_suffix(')')
        .map(str::trim)
}

fn split_lua_css_rgb_channels_and_alpha(
    components: &str,
) -> (std::vec::Vec<&str>, std::option::Option<&str>) {
    if components.contains(',') {
        let components = components.split(',').map(str::trim).collect::<Vec<_>>();
        return (components, None);
    }
    let (channels, alpha) = components
        .split_once('/')
        .map_or((components, None), |(channels, alpha)| {
            (channels, Some(alpha.trim()))
        });
    let channels = channels.split_whitespace().collect::<Vec<_>>();
    (channels, alpha)
}

fn lua_css_rgb_channel_from_query(value: &str) -> Option<u8> {
    if let Some(percent) = value.trim().strip_suffix('%') {
        let percent = parse_finite_f64(percent)?.clamp(0.0, 100.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        return Some((percent * f64::from(u8::MAX) / 100.0) as u8);
    }
    value.trim().parse::<u8>().ok()
}

fn lua_css_alpha_from_query(value: &str) -> Option<u8> {
    if let Some(percent) = value.trim().strip_suffix('%') {
        let percent = parse_finite_f64(percent)?.clamp(0.0, 100.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        return Some((percent * f64::from(u8::MAX) / 100.0) as u8);
    }
    let alpha = parse_finite_f64(value)?.clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some((alpha * f64::from(u8::MAX)) as u8)
}

fn lua_css_hue_degrees_from_query(value: &str) -> Option<f64> {
    let value = value.trim();
    let degrees = if let Some(degrees) = value.strip_suffix("deg") {
        parse_finite_f64(degrees)?
    } else if let Some(turns) = value.strip_suffix("turn") {
        parse_finite_f64(turns)? * 360.0
    } else if let Some(grads) = value.strip_suffix("grad") {
        parse_finite_f64(grads)? * 0.9
    } else if let Some(radians) = value.strip_suffix("rad") {
        parse_finite_f64(radians)? * 180.0 / std::f64::consts::PI
    } else {
        parse_finite_f64(value)?
    };
    Some(degrees.rem_euclid(360.0))
}

fn lua_css_percentage_from_query(value: &str) -> Option<f64> {
    let value = value.trim();
    let percent = value
        .strip_suffix('%')
        .map_or_else(|| parse_finite_f64(value), parse_finite_f64)?;
    Some((percent / 100.0).clamp(0.0, 1.0))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "normalized hue is finite and constrained to the six CSS color sectors"
)]
fn hsl_to_rgb(hue_degrees: f64, saturation: f64, lightness: f64) -> [u8; 3] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_sector = hue_degrees / 60.0;
    let x = chroma * (1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs());
    let [red, green, blue] = match hue_sector as u8 {
        0 => [chroma, x, 0.0],
        1 => [x, chroma, 0.0],
        2 => [0.0, chroma, x],
        3 => [0.0, x, chroma],
        4 => [x, 0.0, chroma],
        _ => [chroma, 0.0, x],
    };
    let m = lightness - chroma / 2.0;
    [
        hsl_channel_to_u8(red + m),
        hsl_channel_to_u8(green + m),
        hsl_channel_to_u8(blue + m),
    ]
}

fn hsl_channel_to_u8(value: f64) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        (value.clamp(0.0, 1.0) * f64::from(u8::MAX)).round() as u8
    }
}

fn hwb_to_rgb(hue_degrees: f64, whiteness: f64, blackness: f64) -> [u8; 3] {
    let white_black_sum = whiteness + blackness;
    if white_black_sum >= 1.0 {
        let gray = whiteness / white_black_sum;
        return [
            round_rgb_component(gray),
            round_rgb_component(gray),
            round_rgb_component(gray),
        ];
    }
    let [red, green, blue] = hsv_to_rgb(hue_degrees, 1.0, 1.0);
    let chroma_scale = 1.0 - white_black_sum;
    [
        round_rgb_component((f64::from(red) / 255.0).mul_add(chroma_scale, whiteness)),
        round_rgb_component((f64::from(green) / 255.0).mul_add(chroma_scale, whiteness)),
        round_rgb_component((f64::from(blue) / 255.0).mul_add(chroma_scale, whiteness)),
    ]
}

fn split_lua_table_key_from_query(key: &str) -> Option<String> {
    split_lua_table_key_from_query_with_static_source(None, key)
}

fn split_lua_table_key_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    key: &str,
) -> Option<String> {
    if let Some(rest) = key.trim().strip_prefix('[') {
        let quoted = lua_trim_start_comments(rest)?;
        if let Some(literal) = lua_quoted_string_literal_from_query(quoted)
            .or_else(|| lua_long_bracket_literal_from_query(quoted))
        {
            let close = lua_trim_start_comments(quoted.get(literal.len()..)?)?;
            if close.trim_start() != "]" {
                return None;
            }
            let value = parse_maybe_quoted_query_text(literal)?;
            return non_empty_spawn_command_option_value(&value).ok();
        }

        let static_source = static_source?;
        let variable = lua_identifier_literal_from_query(quoted)?;
        let close = lua_trim_start_comments(quoted.get(variable.len()..)?)?;
        if close.trim_start() != "]" {
            return None;
        }
        let value = lua_static_string_variable_assignment_before_offset_from_query(
            static_source.source,
            variable,
            static_source.max_start,
        )
        .and_then(parse_maybe_quoted_query_text)?;
        return non_empty_spawn_command_option_value(&value).ok();
    }
    non_empty_spawn_command_option_value(key).ok()
}

fn split_lua_table_key_from_query_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    key: &str,
) -> Option<String> {
    split_lua_table_key_from_query_with_static_source(static_source, key)
        .or_else(|| split_lua_table_key_from_query_with_static_source(outer_static_source, key))
}

fn split_lua_table_string_array(value: &str) -> Option<Vec<String>> {
    split_lua_table_string_array_with_static_source(None, value)
}

fn split_lua_table_string_array_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<String>> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut values = Vec::new();
    let mut indexed_values = BTreeMap::new();
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !values.is_empty() || index == 0 || indexed_values.contains_key(&index) {
                return None;
            }
            indexed_values.insert(
                index,
                parse_maybe_static_query_text(static_source, value.trim())?,
            );
            continue;
        }
        if !indexed_values.is_empty() {
            return None;
        }
        values.push(parse_maybe_static_query_text(static_source, field)?);
    }
    if !indexed_values.is_empty() {
        return (1..=indexed_values.len())
            .map(|index| indexed_values.remove(&index))
            .collect();
    }
    Some(values)
}

fn split_lua_gradient_color_array_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<String>> {
    if let Some(static_source) = static_source
        && let Some(colors) = lua_wezterm_gradient_color_array_from_query(static_source, value)
    {
        return Some(colors);
    }

    split_lua_table_color_expression_array_with_static_source(static_source, value)
}

fn split_lua_table_color_expression_array_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<String>> {
    let color_expression = |query: &str| {
        let query = query.trim();
        if query.starts_with('"') || query.starts_with('\'') || query.starts_with('[') {
            return parse_maybe_quoted_query_text(query);
        }
        if let Some(static_source) = static_source
            && let Some(value) = lua_static_string_assignment_value_before_offset_from_query(
                static_source.source,
                query,
                static_source.max_start,
            )
            .and_then(parse_maybe_quoted_query_text)
        {
            return Some(value);
        }
        (!query.is_empty()).then(|| query.to_owned())
    };
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut values = Vec::new();
    let mut indexed_values = BTreeMap::new();
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !values.is_empty() || index == 0 || indexed_values.contains_key(&index) {
                return None;
            }
            indexed_values.insert(index, color_expression(value)?);
            continue;
        }
        if !indexed_values.is_empty() {
            return None;
        }
        values.push(color_expression(field)?);
    }
    if !indexed_values.is_empty() {
        return (1..=indexed_values.len())
            .map(|index| indexed_values.remove(&index))
            .collect();
    }
    Some(values)
}

fn lua_wezterm_gradient_color_array_from_query(
    static_source: LuaStaticSource<'_>,
    value: &str,
) -> Option<Vec<String>> {
    let value = value.trim();
    let resolved_value = lua_static_wezterm_gradient_color_array_alias_query_from_query(
        static_source.source,
        value,
        static_source.max_start,
    );
    let value = resolved_value.as_deref().unwrap_or(value);
    let rest = lua_function_name_rest_from_query(value, "wezterm.color.gradient")
        .or_else(|| lua_function_name_rest_from_query(value, "wezterm.gradient_colors"))?;

    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let rest = lua_trim_start_comments(rest)?;
    let (gradient, consumed) = lua_wezterm_gradient_color_array_spec_from_query(
        static_source.source,
        rest,
        static_source.max_start,
    )?;
    let rest = lua_trim_start_comments(rest.get(consumed..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    let count_query = lua_trim_start_comments(rest)?.trim_end();
    let count_query = count_query.strip_suffix(')')?.trim_end();
    let count = parse_maybe_static_query_usize(Some(static_source), count_query)?;
    let gradient = native_window_background_gradient_lua_table_from_query(
        static_source.source,
        &gradient,
        static_source.max_start,
    )?;

    background_gradient_color_strings(&gradient.to_render(), count)
}

#[derive(Clone, Copy)]
enum LuaStaticWeztermGradientColorArrayAliasKind {
    ColorGradient,
    GradientColors,
}

impl LuaStaticWeztermGradientColorArrayAliasKind {
    fn normalized_prefix(self) -> &'static str {
        match self {
            Self::ColorGradient => "wezterm.color.gradient",
            Self::GradientColors => "wezterm.gradient_colors",
        }
    }
}

fn lua_static_wezterm_gradient_color_array_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = lua_trim_start_comments(query)?;
    let alias = lua_identifier_literal_from_query(query)?;
    let kind = lua_static_wezterm_gradient_color_array_alias_kind_before_offset(
        source, alias, max_start,
    )??;
    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    if !matches!(rest.chars().next()?, '(') {
        return None;
    }

    Some(format!("{}{}", kind.normalized_prefix(), rest))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_static_wezterm_gradient_color_array_alias_kind_before_offset(
    source: &str,
    alias: &str,
    max_start: usize,
) -> Option<Option<LuaStaticWeztermGradientColorArrayAliasKind>> {
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
        selected = lua_static_wezterm_gradient_color_array_alias_kind_from_value_query(
            source, start, value,
        );
    }

    Some(selected)
}

fn lua_static_wezterm_gradient_color_array_alias_kind_from_value_query(
    source: &str,
    max_start: usize,
    value: &str,
) -> Option<LuaStaticWeztermGradientColorArrayAliasKind> {
    let value = lua_trim_start_comments(value)?;
    let rest = if let Some(rest) = lua_static_wezterm_require_receiver_rest_from_query(value) {
        rest
    } else {
        lua_static_wezterm_receiver_rest_from_query(source, max_start, value)?
    };
    let rest = lua_trim_start_comments(rest)?;
    let static_source = Some(LuaStaticSource { source, max_start });
    let (field, rest) = lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
    if field == "gradient_colors" {
        return lua_static_identifier_value_rest_is_statement_end(rest)
            .then_some(LuaStaticWeztermGradientColorArrayAliasKind::GradientColors);
    }

    if field != "color" {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
    (field == "gradient" && lua_static_identifier_value_rest_is_statement_end(rest))
        .then_some(LuaStaticWeztermGradientColorArrayAliasKind::ColorGradient)
}

fn lua_wezterm_gradient_color_array_spec_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<(String, usize)> {
    if let Some(value) = lua_braced_table_literal_from_query(query) {
        return Some((value.to_owned(), value.len()));
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let value =
        lua_static_table_variable_assignment_before_offset_from_query(source, variable, max_start)?;
    Some((value.to_owned(), variable.len()))
}

fn split_lua_table_u32_array(value: &str) -> Option<Vec<u32>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut indexed_values = BTreeMap::new();
    let mut implicit_index = 1usize;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if index == 0 || indexed_values.contains_key(&index) {
                return None;
            }
            indexed_values.insert(index, lua_unsigned_u32_value_from_query(value.trim())?);
            continue;
        }
        if indexed_values.contains_key(&implicit_index) {
            return None;
        }
        indexed_values.insert(implicit_index, lua_unsigned_u32_value_from_query(field)?);
        implicit_index += 1;
    }
    (1..=indexed_values.len())
        .map(|index| indexed_values.remove(&index))
        .collect()
}

fn split_lua_table_f64_array(value: &str) -> Option<Vec<f64>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut indexed_values = BTreeMap::new();
    let mut implicit_index = 1usize;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if index == 0 || indexed_values.contains_key(&index) {
                return None;
            }
            indexed_values.insert(index, parse_finite_f64(value.trim())?);
            continue;
        }
        if indexed_values.contains_key(&implicit_index) {
            return None;
        }
        indexed_values.insert(implicit_index, parse_finite_f64(field)?);
        implicit_index += 1;
    }
    (1..=indexed_values.len())
        .map(|index| indexed_values.remove(&index))
        .collect()
}

fn split_lua_table_array_index_from_query(key: &str) -> Option<usize> {
    let index = lua_trim_start_comments(key.trim().strip_prefix('[')?)?;
    let literal = lua_unsigned_integer_literal_from_query(index)?;
    let close = lua_trim_start_comments(index.get(literal.len()..)?)?;
    if close.trim_start() != "]" {
        return None;
    }
    literal.parse().ok()
}

fn split_lua_table_top_level_fields(table: &str) -> Option<Vec<&str>> {
    let mut fields = Vec::new();
    let mut depth = 0u32;
    let mut paren_depth = 0u32;
    let mut quote = None;
    let mut start = 0usize;
    let mut escape = false;
    let mut long_bracket_end = None;
    let mut line_comment = false;
    let mut line_comment_field_start = None;
    let mut block_comment_end = None;
    for (index, character) in table.char_indices() {
        if let Some(end) = block_comment_end {
            if index < end {
                continue;
            }
            block_comment_end = None;
            if table[start..end].trim().starts_with("--") {
                start = end;
            }
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
                if line_comment_field_start.is_some() {
                    start = index + character.len_utf8();
                    line_comment_field_start = None;
                }
            }
            continue;
        }
        if let Some(quoted) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == quoted {
                quote = None;
            }
            continue;
        }
        if table[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&table[index + 2..])
            {
                let content_and_rest = &table[index + 2 + content_start..];
                let close_index = content_and_rest.find(&closing)?;
                if depth == 0 && table[start..index].trim().is_empty() {
                    start = index;
                }
                block_comment_end = Some(index + 2 + content_start + close_index + closing.len());
                continue;
            }
            if depth == 0 && table[start..index].trim().is_empty() {
                line_comment_field_start = Some(index);
            }
            line_comment = true;
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '[' => {
                if let Some((content_start, closing)) =
                    parse_lua_long_bracket_delimiters(&table[index..])
                {
                    let content_and_rest = &table[index + content_start..];
                    let close_index = content_and_rest.find(&closing)?;
                    long_bracket_end = Some(index + content_start + close_index + closing.len());
                }
            }
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.saturating_add(1),
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            ',' | ';' if depth == 0 && paren_depth == 0 => {
                fields.push(&table[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quote.is_some() || depth != 0 || paren_depth != 0 {
        return None;
    }
    fields.push(&table[start..]);
    Some(fields)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn split_lua_top_level_arguments(arguments: &str) -> Option<Vec<&str>> {
    let mut values = Vec::new();
    let mut table_depth = 0u32;
    let mut paren_depth = 0u32;
    let mut bracket_depth = 0u32;
    let mut lua_block_depth = 0usize;
    let mut quote = None;
    let mut start = 0usize;
    let mut escape = false;
    let mut long_bracket_end = None;
    let mut line_comment = false;
    let mut block_comment_end = None;

    for (index, character) in arguments.char_indices() {
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
        if let Some(quoted) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == quoted {
                quote = None;
            }
            continue;
        }
        if arguments[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&arguments[index + 2..])
            {
                let content_and_rest = &arguments[index + 2 + content_start..];
                let close_index = content_and_rest.find(&closing)?;
                block_comment_end = Some(index + 2 + content_start + close_index + closing.len());
                continue;
            }
            line_comment = true;
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '[' => {
                let query = arguments.get(index..)?;
                if !lua_bracket_starts_complete_long_string_index(query)
                    && let Some((content_start, closing)) = parse_lua_long_bracket_delimiters(query)
                {
                    let content_and_rest = &arguments[index + content_start..];
                    let close_index = content_and_rest.find(&closing)?;
                    long_bracket_end = Some(index + content_start + close_index + closing.len());
                } else {
                    bracket_depth = bracket_depth.saturating_add(1);
                }
            }
            ']' => bracket_depth = bracket_depth.checked_sub(1)?,
            '{' => table_depth = table_depth.saturating_add(1),
            '}' => table_depth = table_depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.saturating_add(1),
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            ',' if table_depth == 0
                && paren_depth == 0
                && bracket_depth == 0
                && lua_block_depth == 0 =>
            {
                values.push(&arguments[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }

        if lua_source_keyword_at(arguments, index, "function")
            || lua_source_keyword_at(arguments, index, "if")
            || lua_source_keyword_at(arguments, index, "do")
            || lua_source_keyword_at(arguments, index, "repeat")
        {
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }
        if lua_source_keyword_at(arguments, index, "end")
            || lua_source_keyword_at(arguments, index, "until")
        {
            lua_block_depth = lua_block_depth.saturating_sub(1);
        }
    }

    if quote.is_some()
        || table_depth != 0
        || paren_depth != 0
        || bracket_depth != 0
        || lua_block_depth != 0
    {
        return None;
    }
    values.push(&arguments[start..]);
    Some(values)
}

fn lua_parenthesized_argument_list_prefix_from_query(value: &str) -> Option<(&str, &str)> {
    let mut table_depth = 0u32;
    let mut paren_depth = 0u32;
    let mut bracket_depth = 0u32;
    let mut lua_block_depth = 0usize;
    let mut quote = None;
    let mut escape = false;
    let mut long_bracket_end = None;
    let mut line_comment = false;
    let mut block_comment_end = None;

    for (index, character) in value.char_indices() {
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
        if let Some(quoted) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == quoted {
                quote = None;
            }
            continue;
        }
        if value[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&value[index + 2..])
            {
                let content_and_rest = &value[index + 2 + content_start..];
                let close_index = content_and_rest.find(&closing)?;
                block_comment_end = Some(index + 2 + content_start + close_index + closing.len());
                continue;
            }
            line_comment = true;
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            '[' => {
                if let Some((content_start, closing)) =
                    parse_lua_long_bracket_delimiters(&value[index..])
                {
                    let content_and_rest = &value[index + content_start..];
                    let close_index = content_and_rest.find(&closing)?;
                    long_bracket_end = Some(index + content_start + close_index + closing.len());
                } else {
                    bracket_depth = bracket_depth.saturating_add(1);
                }
            }
            ']' => bracket_depth = bracket_depth.checked_sub(1)?,
            '{' => table_depth = table_depth.saturating_add(1),
            '}' => table_depth = table_depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.saturating_add(1),
            ')' if table_depth == 0
                && paren_depth == 0
                && bracket_depth == 0
                && lua_block_depth == 0 =>
            {
                return Some((&value[..index], &value[index + character.len_utf8()..]));
            }
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            _ => {}
        }

        if lua_source_keyword_at(value, index, "function")
            || lua_source_keyword_at(value, index, "if")
            || lua_source_keyword_at(value, index, "do")
            || lua_source_keyword_at(value, index, "repeat")
        {
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }
        if lua_source_keyword_at(value, index, "end")
            || lua_source_keyword_at(value, index, "until")
        {
            lua_block_depth = lua_block_depth.saturating_sub(1);
        }
    }

    None
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn split_lua_table_assignment_from_field(field: &str) -> Option<(&str, &str)> {
    let mut depth = 0u32;
    let mut paren_depth = 0u32;
    let mut bracket_depth = 0u32;
    let mut quote = None;
    let mut escape = false;
    let mut long_bracket_end = None;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut key_end_before_comment = None;
    let mut assignment = None;
    for (index, character) in field.char_indices() {
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
        if let Some(quoted) = quote {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == quoted {
                quote = None;
            }
            continue;
        }
        if depth == 0 && paren_depth == 0 && bracket_depth == 0 && field[index..].starts_with("--")
        {
            if let Some((key_end, value_start)) = assignment {
                if field[value_start..index].trim().is_empty() {
                    if let Some((content_start, closing)) =
                        parse_lua_long_bracket_delimiters(&field[index + 2..])
                    {
                        let content_and_rest = &field[index + 2 + content_start..];
                        let close_index = content_and_rest.find(&closing)?;
                        assignment = Some((
                            key_end,
                            index + 2 + content_start + close_index + closing.len(),
                        ));
                        continue;
                    }
                    line_comment = true;
                    continue;
                }
                if let Some(rest) = lua_trim_start_comments(&field[index..])
                    && lua_expression_continuation_after_comment(rest)
                {
                    if let Some((content_start, closing)) =
                        parse_lua_long_bracket_delimiters(&field[index + 2..])
                    {
                        let content_and_rest = &field[index + 2 + content_start..];
                        let close_index = content_and_rest.find(&closing)?;
                        block_comment_end =
                            Some(index + 2 + content_start + close_index + closing.len());
                        continue;
                    }
                    line_comment = true;
                    continue;
                }
                return Some((&field[..key_end], &field[value_start..index]));
            }
            if key_end_before_comment.is_none() && !field[..index].trim().is_empty() {
                key_end_before_comment = Some(index);
            }
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&field[index + 2..])
            {
                let content_and_rest = &field[index + 2 + content_start..];
                let close_index = content_and_rest.find(&closing)?;
                block_comment_end = Some(index + 2 + content_start + close_index + closing.len());
                continue;
            }
            line_comment = true;
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '[' => {
                if let Some((content_start, closing)) =
                    parse_lua_long_bracket_delimiters(&field[index..])
                {
                    let content_and_rest = &field[index + content_start..];
                    let close_index = content_and_rest.find(&closing)?;
                    long_bracket_end = Some(index + content_start + close_index + closing.len());
                } else {
                    bracket_depth = bracket_depth.saturating_add(1);
                }
            }
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.checked_sub(1)?,
            '(' => paren_depth = paren_depth.saturating_add(1),
            ')' => paren_depth = paren_depth.checked_sub(1)?,
            '=' if depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                let key_end = key_end_before_comment.unwrap_or(index);
                assignment = Some((key_end, index + character.len_utf8()));
            }
            _ => {}
        }
    }
    let (key_end, value_start) = assignment?;
    Some((
        &field[..key_end],
        lua_trim_start_comments(&field[value_start..])?,
    ))
}

fn lua_expression_continuation_after_comment(rest: &str) -> bool {
    let rest = rest.trim_start();
    rest.starts_with('\'')
        || rest.starts_with('"')
        || rest.starts_with('{')
        || rest.starts_with('(')
        || rest.starts_with('.')
        || rest.starts_with('[')
        || lua_long_bracket_literal_from_query(rest).is_some()
}

fn lua_identifier_rest_has_expression_continuation_after_comment(rest: &str) -> bool {
    rest.trim_start().starts_with("--")
        && lua_trim_start_comments(rest).is_some_and(lua_expression_continuation_after_comment)
}

fn split_pane_table_size_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowSplitPaneSize> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let (key, value) = split_lua_table_assignment_from_field(table)?;
    let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
    let value = value.trim().trim_end_matches(',').trim();
    let amount = if let Some(static_source) = static_source {
        lua_static_number_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
            lua_unsigned_integer_literal_from_query,
        )
        .map(str::to_owned)
        .or_else(|| parse_maybe_quoted_query_text(value))?
    } else {
        parse_maybe_quoted_query_text(value)?
    }
    .parse()
    .ok()?;
    match key.to_ascii_lowercase().as_str() {
        "percent" => Some(WindowSplitPaneSize::Percent(amount)),
        "cells" => Some(WindowSplitPaneSize::Cells(amount)),
        _ => None,
    }
}

fn split_horizontal_command_from_query(query: &str) -> Option<WindowSpawnCommandQuery> {
    split_horizontal_options_from_query(query).and_then(|options| options.command)
}

fn split_vertical_command_from_query(query: &str) -> Option<WindowSpawnCommandQuery> {
    split_vertical_options_from_query(query).and_then(|options| options.command)
}

fn split_pane_options_from_query(
    query: &str,
    prefix: &str,
    direction: SplitDirection,
) -> Option<WindowSplitPaneOptions> {
    let command = strip_query_prefix_from_any(query, &[prefix])?;
    if command.trim_start().starts_with('=') || command.trim_start().starts_with('{') {
        return None;
    }
    split_pane_options_from_rest(command, direction)
}

fn split_pane_structured_options_from_query(query: &str) -> Option<WindowSplitPaneOptions> {
    let command = strip_query_prefix_from_any(query, &["splitpane=", "splitpane "])?;
    let words = command_palette_query_words(command)?;
    let has_direction = words
        .iter()
        .any(|word| query_assignment_value_from_token(word, &["direction"]).is_some())
        || words
            .windows(2)
            .any(|words| words[0].eq_ignore_ascii_case("direction"));
    if !has_direction {
        return None;
    }
    split_pane_options_from_rest(command, SplitDirection::Right)
}

fn split_pane_options_from_rest(
    command: &str,
    fallback_direction: SplitDirection,
) -> Option<WindowSplitPaneOptions> {
    let words = command_palette_query_words(command)?;
    let mut words = words.iter().map(String::as_str).peekable();
    let (split_options, spawn_options) =
        parse_split_pane_spawn_command_query_options(&mut words).ok()?;
    let direction = split_options.direction.unwrap_or(fallback_direction);
    let command_words = words.collect::<Vec<_>>();
    let command = split_pane_command_from_words(&command_words, &spawn_options);
    let command_options = if command.is_none()
        && split_pane_command_options_supported_without_program(&spawn_options)
    {
        Some(spawn_options.clone())
    } else {
        None
    };

    if command.is_none()
        && split_options.size.is_none()
        && !split_options.top_level
        && spawn_options.domain.is_none()
        && command_options.is_none()
    {
        return None;
    }

    Some(WindowSplitPaneOptions {
        direction,
        domain: spawn_options.domain.clone(),
        command,
        command_options,
        size: split_options.size,
        top_level: split_options.top_level,
    })
}

fn split_pane_command_from_words(
    words: &[&str],
    options: &WindowSpawnCommandQueryOptions,
) -> Option<WindowSpawnCommandQuery> {
    let mut options = options.clone();
    let mut command_domain = None;
    let (program, args) = match words {
        [] => return None,
        ["command", rest @ ..] => {
            let mut index = 0;
            while index < rest.len() {
                if let Some(value) = query_text_assignment_value_from_token(rest[index], &["cwd"]) {
                    options.cwd = Some(non_empty_spawn_command_option_value(value).ok()?);
                    index += 1;
                    continue;
                }
                if let Some(value) =
                    query_text_assignment_value_from_token(rest[index], &["domain"])
                {
                    command_domain = Some(spawn_command_domain_from_query(value)?);
                    index += 1;
                    continue;
                }
                if let Some(value) = query_text_assignment_value_from_token(
                    rest[index],
                    &["set_environment_variables", "set-environment-variables"],
                ) {
                    let (name, value) = spawn_command_environment_from_query(value).ok()?;
                    options.environment.insert(name, value);
                    index += 1;
                    continue;
                }
                break;
            }
            let (program, args) = rest[index..].split_first()?;
            let program = query_assignment_value_from_token(program, &["args"])?;
            (program, args)
        }
        [program, args @ ..] => (
            query_assignment_value_from_token(program, &["command"]).unwrap_or(program),
            args,
        ),
    };
    Some(WindowSpawnCommandQuery {
        label: None,
        program: program.to_owned(),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        cwd: options.cwd.clone(),
        environment: options.environment.clone(),
        domain: command_domain,
        window_position: options.window_position.clone(),
    })
}

fn split_pane_command_options_supported_without_program(
    options: &WindowSpawnCommandQueryOptions,
) -> bool {
    options.window_position.is_none()
        && (options.cwd.is_some() || !options.environment.is_empty() || options.domain.is_some())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowSplitPaneQueryOptions {
    direction: Option<SplitDirection>,
    size: Option<WindowSplitPaneSize>,
    top_level: bool,
}

fn parse_split_pane_spawn_command_query_options<'a, I>(
    words: &mut std::iter::Peekable<I>,
) -> Result<(WindowSplitPaneQueryOptions, WindowSpawnCommandQueryOptions), ()>
where
    I: Iterator<Item = &'a str> + Clone,
{
    let mut split_options = WindowSplitPaneQueryOptions::default();
    let mut spawn_options = WindowSpawnCommandQueryOptions::default();
    loop {
        if parse_split_pane_query_option(words, &mut split_options)? {
            continue;
        }
        if parse_spawn_command_query_option(words, &mut spawn_options)? {
            continue;
        }
        break;
    }
    Ok((split_options, spawn_options))
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn parse_split_pane_query_option<'a, I>(
    words: &mut std::iter::Peekable<I>,
    options: &mut WindowSplitPaneQueryOptions,
) -> Result<bool, ()>
where
    I: Iterator<Item = &'a str> + Clone,
{
    match words.peek().copied() {
        Some(option) if option.eq_ignore_ascii_case("--percent") => {
            words.next();
            let value = words.next().ok_or(())?;
            options.size = Some(WindowSplitPaneSize::Percent(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if starts_with_ascii_case_insensitive(option, "--percent=") => {
            let value = strip_query_prefix_from_any(option, &["--percent="]).ok_or(())?;
            words.next();
            options.size = Some(WindowSplitPaneSize::Percent(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("--cells") => {
            words.next();
            let value = words.next().ok_or(())?;
            options.size = Some(WindowSplitPaneSize::Cells(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if starts_with_ascii_case_insensitive(option, "--cells=") => {
            let value = strip_query_prefix_from_any(option, &["--cells="]).ok_or(())?;
            words.next();
            options.size = Some(WindowSplitPaneSize::Cells(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if query_assignment_value_from_token(option, &["percent"]).is_some() => {
            let value = query_assignment_value_from_token(option, &["percent"]).ok_or(())?;
            words.next();
            options.size = Some(WindowSplitPaneSize::Percent(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("percent") => {
            words.next();
            let value = words.next().ok_or(())?;
            options.size = Some(WindowSplitPaneSize::Percent(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if query_assignment_value_from_token(option, &["cells"]).is_some() => {
            let value = query_assignment_value_from_token(option, &["cells"]).ok_or(())?;
            words.next();
            options.size = Some(WindowSplitPaneSize::Cells(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("cells") => {
            words.next();
            let value = words.next().ok_or(())?;
            options.size = Some(WindowSplitPaneSize::Cells(value.parse().map_err(|_| ())?));
            Ok(true)
        }
        Some(option) if query_assignment_value_from_token(option, &["direction"]).is_some() => {
            let value = query_assignment_value_from_token(option, &["direction"]).ok_or(())?;
            words.next();
            if options.direction.is_some() {
                return Err(());
            }
            options.direction = Some(split_pane_direction_from_query(value).ok_or(())?);
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("direction") => {
            words.next();
            let value = words.next().ok_or(())?;
            if options.direction.is_some() {
                return Err(());
            }
            options.direction = Some(split_pane_direction_from_query(value).ok_or(())?);
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("--top-level") => {
            words.next();
            options.top_level = true;
            Ok(true)
        }
        Some(option) if starts_with_ascii_case_insensitive(option, "--top-level=") => {
            let value = strip_query_prefix_from_any(option, &["--top-level="]).ok_or(())?;
            words.next();
            options.top_level = bool_from_query(value).ok_or(())?;
            Ok(true)
        }
        Some(option) => {
            if let Some(value) =
                query_assignment_value_from_token(option, &["top_level", "top-level"])
            {
                words.next();
                options.top_level = bool_from_query(value).ok_or(())?;
            } else if option.eq_ignore_ascii_case("top_level")
                || option.eq_ignore_ascii_case("top-level")
            {
                words.next();
                let value = words.next().ok_or(())?;
                options.top_level = bool_from_query(value).ok_or(())?;
            } else if !options.top_level && option.eq_ignore_ascii_case("top") {
                let mut lookahead = words.clone();
                lookahead.next();
                let Some(level) = lookahead.next() else {
                    return Ok(false);
                };
                let (value, consume_value_token) = if level.eq_ignore_ascii_case("level") {
                    (lookahead.next().ok_or(())?, true)
                } else if let Some(value) = query_assignment_value_from_token(level, &["level"]) {
                    (value, false)
                } else {
                    return Ok(false);
                };
                words.next();
                words.next();
                if consume_value_token {
                    words.next();
                }
                options.top_level = bool_from_query(value).ok_or(())?;
            } else {
                return Ok(false);
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn split_pane_direction_from_query(direction: &str) -> Option<SplitDirection> {
    let normalized = direction
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "right" => Some(SplitDirection::Right),
        "down" | "bottom" => Some(SplitDirection::Down),
        "left" => Some(SplitDirection::Left),
        "up" | "top" => Some(SplitDirection::Up),
        _ => None,
    }
}

fn quick_select_pattern_from_query(query: &str) -> Option<String> {
    if let Some(value) = quick_select_assignment_value_from_query(query, &["pattern"]) {
        return parse_maybe_quoted_query_text(&value);
    }

    if let Some(value) = quick_select_pattern_word_value_from_query(query, "pattern") {
        return parse_maybe_quoted_query_text(&value);
    }

    strip_query_prefix_from_any(
        query,
        &[
            "quick select pattern ",
            "quickselectargs pattern ",
            "quickselect pattern ",
        ],
    )
    .and_then(|pattern| quick_select_text_before_next_field(pattern, quick_select_next_fields()))
    .and_then(parse_maybe_quoted_query_text)
}

fn quick_select_patterns_from_query(query: &str) -> Option<Vec<String>> {
    if let Some(value) = quick_select_assignment_value_from_query(query, &["patterns"]) {
        let patterns = split_unquoted_query_semicolons(&value)
            .into_iter()
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
            .map(parse_maybe_quoted_query_text)
            .collect::<Option<Vec<_>>>()?;

        return (!patterns.is_empty()).then_some(patterns);
    }

    if let Some(value) = quick_select_pattern_word_value_from_query(query, "patterns") {
        let patterns = split_unquoted_query_semicolons(&value)
            .into_iter()
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
            .map(parse_maybe_quoted_query_text)
            .collect::<Option<Vec<_>>>()?;

        return (!patterns.is_empty()).then_some(patterns);
    }

    let value = strip_query_prefix_from_any(
        query,
        &[
            "quick select patterns ",
            "quickselectargs patterns ",
            "quickselect patterns ",
        ],
    )?;
    let value = quick_select_text_before_next_field(value, quick_select_next_fields())?;
    let patterns = split_unquoted_query_semicolons(value)
        .into_iter()
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .map(parse_maybe_quoted_query_text)
        .collect::<Option<Vec<_>>>()?;

    (!patterns.is_empty()).then_some(patterns)
}

fn quick_select_pattern_word_value_from_query(query: &str, key: &str) -> Option<String> {
    let rest = strip_query_prefix_from_any(query, &["quickselectargs=", "quickselectargs "])?;
    let pattern = quick_select_text_after_word(rest, key)?;
    quick_select_text_before_next_field(pattern, quick_select_next_fields()).map(str::to_owned)
}

fn quick_select_alphabet_from_query(query: &str) -> Option<String> {
    if let Some(value) = quick_select_assignment_value_from_query(query, &["alphabet"]) {
        return parse_maybe_quoted_query_text(&value);
    }

    if let Some(value) = quick_select_word_value_from_query(query, "alphabet") {
        return parse_maybe_quoted_query_text(&value);
    }

    strip_query_prefix_from_any(
        query,
        &[
            "quick select alphabet ",
            "quickselectargs alphabet ",
            "quickselect alphabet ",
        ],
    )
    .and_then(parse_maybe_quoted_query_text)
}

fn quick_select_word_value_from_query(query: &str, key: &str) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "quick select=",
            "quick select ",
            "quickselectargs=",
            "quickselectargs ",
            "quickselect=",
            "quickselect ",
        ],
    )
    .and_then(|rest| quick_select_word_value_from_text(rest, key))
}

fn quick_select_word_value_from_text(text: &str, key: &str) -> Option<String> {
    let tokens = command_palette_query_words(text)?;
    let key_index = tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case(key))?;
    tokens.get(key_index + 1).cloned()
}

fn quick_select_label_from_query(query: &str) -> Option<String> {
    if let Some(value) = quick_select_assignment_value_from_query(query, &["label"]) {
        return parse_maybe_quoted_query_text(&value);
    }

    if let Some(value) = quick_select_label_word_value_from_query(query) {
        return parse_maybe_quoted_query_text(&value);
    }

    let label = strip_query_prefix_from_any(
        query,
        &[
            "quick select label ",
            "quickselectargs label ",
            "quickselect label ",
        ],
    )?;
    quick_select_label_text_before_next_field(label).and_then(parse_maybe_quoted_query_text)
}

fn quick_select_label_text_before_next_field(label: &str) -> Option<&str> {
    quick_select_text_before_next_field(label, quick_select_next_fields())
}

fn quick_select_next_fields() -> &'static [&'static [&'static str]] {
    &[
        &["action"],
        &["alphabet"],
        &["label"],
        &["pattern"],
        &["patterns"],
        &["skip", "action", "on", "paste"],
        &["skip_action_on_paste"],
        &["skip-action-on-paste"],
        &["scope", "lines"],
        &["scope_lines"],
        &["scope-lines"],
    ]
}

fn quick_select_label_word_value_from_query(query: &str) -> Option<String> {
    let rest = strip_query_prefix_from_any(query, &["quickselectargs=", "quickselectargs "])?;
    let label = quick_select_text_after_word(rest, "label")?;
    quick_select_text_before_next_field(label, quick_select_next_fields()).map(str::to_owned)
}

fn quick_select_text_after_word<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let words = command_palette_query_word_spans(text)?;
    let (_, _, end) = words
        .iter()
        .find(|(word, _, _)| word.eq_ignore_ascii_case(key))?;
    text.get(*end..)
        .map(str::trim_start)
        .filter(|value| !value.is_empty())
}

fn quick_select_text_before_next_field<'a>(text: &'a str, fields: &[&[&str]]) -> Option<&'a str> {
    let words = command_palette_query_word_spans(text)?;
    if words.is_empty() {
        return None;
    }

    for index in 1..words.len() {
        for field in fields {
            if index + field.len() <= words.len()
                && field
                    .iter()
                    .zip(words[index..].iter())
                    .all(|(field_word, (word, _, _))| {
                        quick_select_field_word_matches(word, field_word)
                    })
            {
                return text.get(..words[index].1).map(str::trim_end);
            }
        }
    }

    Some(text)
}

fn quick_select_field_word_matches(word: &str, field_word: &str) -> bool {
    word.eq_ignore_ascii_case(field_word)
        || word
            .get(..field_word.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(field_word))
            && word.as_bytes().get(field_word.len()) == Some(&b'=')
}

fn quick_select_assignment_value_from_query(query: &str, keys: &[&str]) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "quick select=",
            "quick select ",
            "quickselectargs=",
            "quickselectargs ",
            "quickselect=",
            "quickselect ",
        ],
    )
    .and_then(|rest| quick_select_assignment_value_from_text(rest, keys))
}

fn quick_select_assignment_value_from_text(text: &str, keys: &[&str]) -> Option<String> {
    command_palette_query_words(text)
        .and_then(|tokens| {
            tokens.into_iter().find_map(|token| {
                query_assignment_value_from_token(&token, keys).map(str::to_owned)
            })
        })
        .or_else(|| query_assignment_value_from_token(text.trim(), keys).map(str::to_owned))
}

fn quick_select_action_from_query(query: &str) -> Option<WindowQuickSelectAction> {
    let action = quick_select_action_query_text(query)?;
    let action = quick_select_action_suffix_stripped(&action);
    let action = quick_select_text_before_next_field(action, quick_select_next_fields())?;
    let action = parse_maybe_quoted_query_text(action)?;
    quick_select_action_from_value(&action)
}

fn quick_select_action_from_value(action: &str) -> Option<WindowQuickSelectAction> {
    if let Some(destination) =
        strip_query_prefix_from_any(action, &["copy to=", "copy to ", "copy=", "copy "])
    {
        return copy_destination_from_query(destination).map(WindowQuickSelectAction::CopyTo);
    }
    if let Some(action) = quick_select_key_assignment_action_from_value(action) {
        return Some(action);
    }
    let normalized = action
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "copytoclipboard" | "copyclipboard" => Some(WindowQuickSelectAction::CopyTo(
            WindowCopyDestination::Clipboard,
        )),
        "copytoprimaryselection" | "copyprimaryselection" => Some(WindowQuickSelectAction::CopyTo(
            WindowCopyDestination::PrimarySelection,
        )),
        "copytoclipboardandprimaryselection" | "copyclipboardandprimaryselection" => Some(
            WindowQuickSelectAction::CopyTo(WindowCopyDestination::ClipboardAndPrimarySelection),
        ),
        "openuri" | "openurl" => Some(WindowQuickSelectAction::OpenUri),
        _ => None,
    }
}

fn quick_select_action_from_window_command(
    command: WindowCommand,
) -> Option<WindowQuickSelectAction> {
    match command {
        WindowCommand::CompleteSelectionTo(destination)
        | WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(destination)
        | WindowCommand::CopyTo(destination) => Some(WindowQuickSelectAction::CopyTo(destination)),
        WindowCommand::CopyToClipboard | WindowCommand::Copy => Some(
            WindowQuickSelectAction::CopyTo(WindowCopyDestination::Clipboard),
        ),
        WindowCommand::CopyToPrimarySelection => Some(WindowQuickSelectAction::CopyTo(
            WindowCopyDestination::PrimarySelection,
        )),
        WindowCommand::CompleteSelection
        | WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor
        | WindowCommand::CopyToClipboardAndPrimarySelection => Some(
            WindowQuickSelectAction::CopyTo(WindowCopyDestination::ClipboardAndPrimarySelection),
        ),
        WindowCommand::Paste | WindowCommand::PasteFromClipboard => Some(
            WindowQuickSelectAction::PasteFrom(WindowPasteSource::Clipboard),
        ),
        WindowCommand::PastePrimarySelection | WindowCommand::PasteFromPrimarySelection => Some(
            WindowQuickSelectAction::PasteFrom(WindowPasteSource::PrimarySelection),
        ),
        WindowCommand::PasteFrom(source) => Some(WindowQuickSelectAction::PasteFrom(source)),
        WindowCommand::SendString(value) => Some(WindowQuickSelectAction::SendString(value)),
        WindowCommand::SendKey(send_key) => Some(WindowQuickSelectAction::SendKey(send_key)),
        WindowCommand::EmitEvent(event) => Some(WindowQuickSelectAction::EmitEvent(event)),
        WindowCommand::Multiple(commands) => Some(WindowQuickSelectAction::Multiple(commands)),
        WindowCommand::ActivateKeyTable(key_table) => {
            Some(WindowQuickSelectAction::ActivateKeyTable(key_table))
        }
        WindowCommand::PopKeyTable => Some(WindowQuickSelectAction::PopKeyTable),
        WindowCommand::ClearKeyTableStack => Some(WindowQuickSelectAction::ClearKeyTableStack),
        WindowCommand::Nop => Some(WindowQuickSelectAction::Nop),
        _ => None,
    }
}

fn quick_select_key_assignment_action_from_value(action: &str) -> Option<WindowQuickSelectAction> {
    if let Some(action) = quick_select_wrapped_key_assignment_action_from_value(action) {
        return Some(action);
    }

    let indexed_action;
    let action = if let Some(action) = strip_wezterm_action_prefix(action) {
        action
    } else if let Some(action) = strip_wezterm_action_index_prefix(action) {
        indexed_action = action;
        indexed_action.as_str()
    } else {
        action
    };
    if let Some(action) = quick_select_no_arg_key_assignment_action_from_value(action) {
        return Some(action);
    }
    if let Some(action) = quick_select_complete_selection_action_from_value(action) {
        return Some(action);
    }
    if let Some(source) = paste_source_command_from_query(action) {
        return Some(WindowQuickSelectAction::PasteFrom(source));
    }
    if let Some(command) = command_palette_structured_query_command(action)
        && let Some(action) = quick_select_action_from_window_command(command)
    {
        return Some(action);
    }

    copy_destination_command_from_query(action).map(WindowQuickSelectAction::CopyTo)
}

fn quick_select_complete_selection_action_from_value(
    action: &str,
) -> Option<WindowQuickSelectAction> {
    match complete_selection_command_from_query(action)? {
        WindowCommand::CompleteSelectionTo(destination)
        | WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(destination) => {
            Some(WindowQuickSelectAction::CopyTo(destination))
        }
        _ => None,
    }
}

fn quick_select_no_arg_key_assignment_action_from_value(
    action: &str,
) -> Option<WindowQuickSelectAction> {
    let action = strip_zero_arg_lua_function_call_from_query(action).unwrap_or(action);
    match normalized_action_name_query(action).as_str() {
        "nop" => Some(WindowQuickSelectAction::Nop),
        "popkeytable" => Some(WindowQuickSelectAction::PopKeyTable),
        "clearkeytablestack" => Some(WindowQuickSelectAction::ClearKeyTableStack),
        _ => None,
    }
}

fn quick_select_wrapped_key_assignment_action_from_value(
    action: &str,
) -> Option<WindowQuickSelectAction> {
    let table = strip_wezterm_action_table_wrapper_from_query(action)?;
    let mut fields = split_lua_table_top_level_fields(table)?
        .into_iter()
        .map(str::trim)
        .filter(|field| !field.is_empty());
    let field = fields.next()?;
    if fields.next().is_some() {
        return None;
    }

    let (name, value) = split_lua_table_assignment_from_field(field)?;
    let name = split_lua_table_key_from_query(name.trim())?;
    match normalized_action_name_query(&name).as_str() {
        "copyto" => copy_destination_from_query(value).map(WindowQuickSelectAction::CopyTo),
        "pastefrom" => paste_source_from_query(value).map(WindowQuickSelectAction::PasteFrom),
        _ => None,
    }
}

fn quick_select_action_query_text(query: &str) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "quick select=",
            "quick select ",
            "quickselectargs=",
            "quickselectargs ",
            "quickselect=",
            "quickselect ",
        ],
    )
    .and_then(|rest| {
        quick_select_text_assignment_value_from_text(rest, &["action"])
            .or_else(|| quick_select_action_word_value_from_text(rest))
    })
    .or_else(|| {
        strip_query_prefix_from_any(
            query,
            &[
                "quick select action ",
                "quickselectargs action ",
                "quickselect action ",
            ],
        )
        .map(str::to_owned)
    })
}

fn quick_select_action_word_value_from_text(text: &str) -> Option<String> {
    let tokens = command_palette_query_words(text)?;
    let action_index = tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case("action"))?;
    let action_tokens = tokens.get(action_index + 1..)?;
    (!action_tokens.is_empty()).then(|| action_tokens.join(" "))
}

fn quick_select_text_assignment_value_from_text(text: &str, keys: &[&str]) -> Option<String> {
    command_palette_query_words(text)
        .and_then(|tokens| {
            tokens.into_iter().find_map(|token| {
                query_text_assignment_value_from_token(&token, keys).map(str::to_owned)
            })
        })
        .or_else(|| query_text_assignment_value_from_token(text.trim(), keys).map(str::to_owned))
}

fn quick_select_action_suffix_stripped(action: &str) -> &str {
    let mut action = action.trim();
    loop {
        let stripped_scope = quick_select_scope_lines_suffix(action).map(|(action, _)| action);
        let stripped_skip =
            quick_select_skip_action_on_paste_suffix_stripped(stripped_scope.unwrap_or(action));
        let stripped = stripped_skip.or(stripped_scope).unwrap_or(action).trim();
        if stripped.len() == action.len() {
            return action;
        }
        action = stripped;
    }
}

fn quick_select_skip_action_on_paste_suffix_stripped(action: &str) -> Option<&str> {
    [
        " skip action on paste true",
        " skip action on paste false",
        " skip action on paste=true",
        " skip action on paste=false",
        " skip action on paste",
        " skip_action_on_paste true",
        " skip_action_on_paste false",
        " skip_action_on_paste=true",
        " skip_action_on_paste=false",
        " skip_action_on_paste",
        " skip-action-on-paste true",
        " skip-action-on-paste false",
        " skip-action-on-paste=true",
        " skip-action-on-paste=false",
        " skip-action-on-paste",
    ]
    .into_iter()
    .find_map(|suffix| strip_ascii_case_insensitive_suffix(action, suffix))
    .map(str::trim)
}

fn quick_select_scope_lines_suffix(action: &str) -> Option<(&str, usize)> {
    let lower = action.to_ascii_lowercase();
    [
        " scope lines=",
        " scope_lines=",
        " scope-lines=",
        " scope lines ",
        " scope_lines ",
        " scope-lines ",
    ]
    .into_iter()
    .find_map(|marker| {
        let index = lower.rfind(marker)?;
        let value = action.get(index + marker.len()..)?.trim();
        let scope_lines = value
            .split_once(char::is_whitespace)
            .map_or(value, |(value, _)| value)
            .parse()
            .ok()?;
        Some((action.get(..index)?.trim_end(), scope_lines))
    })
}

fn strip_ascii_case_insensitive_suffix<'a>(value: &'a str, suffix: &str) -> Option<&'a str> {
    let start = value.len().checked_sub(suffix.len())?;
    value[start..]
        .eq_ignore_ascii_case(suffix)
        .then_some(&value[..start])
}

fn quick_select_skip_action_on_paste_from_query(query: &str) -> Option<bool> {
    if let Some(value) = quick_select_assignment_value_from_query(
        query,
        &[
            "skip action on paste",
            "skip_action_on_paste",
            "skip-action-on-paste",
        ],
    ) {
        return bool_from_query(&value);
    }

    if let Some(value) = quick_select_bare_skip_action_on_paste_from_query(query) {
        return Some(value);
    }

    let action = quick_select_action_query_text(query)?;
    if action.ends_with(" skip action on paste false")
        || action.ends_with(" skip action on paste=false")
        || action.ends_with(" skip_action_on_paste false")
        || action.ends_with(" skip_action_on_paste=false")
        || action.ends_with(" skip-action-on-paste false")
        || action.ends_with(" skip-action-on-paste=false")
    {
        return Some(false);
    }
    if action.ends_with(" skip action on paste")
        || action.ends_with(" skip action on paste true")
        || action.ends_with(" skip action on paste=true")
        || action.ends_with(" skip_action_on_paste")
        || action.ends_with(" skip_action_on_paste true")
        || action.ends_with(" skip_action_on_paste=true")
        || action.ends_with(" skip-action-on-paste")
        || action.ends_with(" skip-action-on-paste true")
        || action.ends_with(" skip-action-on-paste=true")
    {
        return Some(true);
    }
    None
}

fn quick_select_bare_skip_action_on_paste_from_query(query: &str) -> Option<bool> {
    let rest = strip_query_prefix_from_any(
        query,
        &[
            "quick select=",
            "quick select ",
            "quickselectargs=",
            "quickselectargs ",
            "quickselect=",
            "quickselect ",
        ],
    )?;
    let tokens = command_palette_query_words(rest)?;
    tokens.iter().enumerate().find_map(|(index, token)| {
        if token.eq_ignore_ascii_case("skip_action_on_paste")
            || token.eq_ignore_ascii_case("skip-action-on-paste")
        {
            return tokens
                .get(index + 1)
                .and_then(|token| bool_from_query(token))
                .or(Some(true));
        }

        if let Some([skip, action, on, paste]) = tokens.get(index..index + 4)
            && skip.eq_ignore_ascii_case("skip")
            && action.eq_ignore_ascii_case("action")
            && on.eq_ignore_ascii_case("on")
        {
            if paste.eq_ignore_ascii_case("paste") {
                return tokens
                    .get(index + 4)
                    .and_then(|token| bool_from_query(token))
                    .or(Some(true));
            }
            if let Some(value) = strip_query_prefix_from_any(paste, &["paste="]) {
                return bool_from_query(value);
            }
        }

        None
    })
}

fn quick_select_scope_lines_from_query(query: &str) -> Option<usize> {
    if let Some(value) = quick_select_assignment_value_from_query(
        query,
        &["scope lines", "scope_lines", "scope-lines"],
    ) {
        return value.parse().ok();
    }

    if let Some(scope_lines) = quick_select_scope_lines_word_value_from_query(query) {
        return Some(scope_lines);
    }

    if let Some(scope_lines) = quick_select_action_query_text(query)
        .as_deref()
        .and_then(quick_select_scope_lines_suffix)
        .map(|(_, scope_lines)| scope_lines)
    {
        return Some(scope_lines);
    }

    strip_query_prefix_from_any(
        query,
        &[
            "quick select scope lines ",
            "quickselectargs scope lines ",
            "quickselect scope lines ",
            "quick select scope_lines ",
            "quickselectargs scope_lines ",
            "quickselect scope_lines ",
            "quick select scope-lines ",
            "quickselectargs scope-lines ",
            "quickselect scope-lines ",
        ],
    )?
    .trim()
    .parse()
    .ok()
}

fn quick_select_scope_lines_word_value_from_query(query: &str) -> Option<usize> {
    let rest = strip_query_prefix_from_any(query, &["quickselectargs=", "quickselectargs "])?;
    let tokens = command_palette_query_words(rest)?;
    tokens.iter().enumerate().find_map(|(index, token)| {
        if token.eq_ignore_ascii_case("scope_lines") || token.eq_ignore_ascii_case("scope-lines") {
            return tokens.get(index + 1)?.parse().ok();
        }

        if token.eq_ignore_ascii_case("scope")
            && tokens.get(index + 1)?.eq_ignore_ascii_case("lines")
        {
            return tokens.get(index + 2)?.parse().ok();
        }

        if let Some(value) = strip_query_prefix_from_any(token, &["scope_lines=", "scope-lines="]) {
            return value.parse().ok();
        }

        if token.eq_ignore_ascii_case("scope") {
            let lines = tokens.get(index + 1)?;
            if let Some(value) = strip_query_prefix_from_any(lines, &["lines="]) {
                return value.parse().ok();
            }
        }

        None
    })
}

fn quick_select_options_from_query(query: &str) -> WindowQuickSelectOptions {
    if let Some(options) = quick_select_lua_table_from_query(query) {
        return options;
    }

    let mut patterns = quick_select_patterns_from_query(query);
    if patterns.is_none() {
        patterns = quick_select_pattern_from_query(query).map(|pattern| vec![pattern]);
    }
    WindowQuickSelectOptions {
        patterns,
        alphabet: quick_select_alphabet_from_query(query),
        label: quick_select_label_from_query(query),
        action: quick_select_action_from_query(query),
        skip_action_on_paste: quick_select_skip_action_on_paste_from_query(query).unwrap_or(false),
        scope_lines: quick_select_scope_lines_from_query(query),
    }
}

fn quick_select_lua_table_from_query(query: &str) -> Option<WindowQuickSelectOptions> {
    quick_select_lua_table_from_query_with_static_source(None, query)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn quick_select_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowQuickSelectOptions> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    let value = strip_lua_function_call_from_query(query, "quickselectargs")
        .or_else(|| strip_query_table_assignment_from_prefix(query, "quickselectargs="))?;
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut options = WindowQuickSelectOptions::default();
    let mut parsed = false;
    let mut parsed_patterns = false;
    let mut parsed_alphabet = false;
    let mut parsed_label = false;
    let mut parsed_action = false;
    let mut parsed_skip_action_on_paste = false;
    let mut parsed_scope_lines = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();

        match normalized_quick_select_lua_field(&name).as_str() {
            "pattern" => {
                if parsed_patterns {
                    return None;
                }
                options.patterns = Some(vec![parse_maybe_static_query_text(static_source, value)?]);
                parsed_patterns = true;
            }
            "patterns" => {
                if parsed_patterns {
                    return None;
                }
                let patterns = if value.starts_with('{') {
                    split_lua_table_string_array_with_static_source(static_source, value)?
                } else if let Some(patterns) =
                    split_lua_table_string_array_with_static_source(static_source, value)
                {
                    patterns
                } else {
                    let value = parse_maybe_static_query_text(static_source, value)?;
                    split_unquoted_query_semicolons(&value)
                        .into_iter()
                        .map(str::trim)
                        .filter(|pattern| !pattern.is_empty())
                        .map(parse_maybe_quoted_query_text)
                        .collect::<Option<Vec<_>>>()?
                };
                if patterns.is_empty() {
                    return None;
                }
                options.patterns = Some(patterns);
                parsed_patterns = true;
            }
            "alphabet" => {
                if parsed_alphabet {
                    return None;
                }
                options.alphabet = Some(parse_maybe_static_query_text(static_source, value)?);
                parsed_alphabet = true;
            }
            "label" => {
                if parsed_label {
                    return None;
                }
                options.label = Some(parse_maybe_static_query_text(static_source, value)?);
                parsed_label = true;
            }
            "action" => {
                if parsed_action {
                    return None;
                }
                options.action = Some(quick_select_action_callback_or_key_assignment_from_query(
                    static_source,
                    value,
                )?);
                parsed_action = true;
            }
            "skipactiononpaste" => {
                if parsed_skip_action_on_paste {
                    return None;
                }
                options.skip_action_on_paste = parse_maybe_static_query_bool(static_source, value)?;
                parsed_skip_action_on_paste = true;
            }
            "scopelines" => {
                if parsed_scope_lines {
                    return None;
                }
                let scope_lines = if let Some(static_source) = static_source {
                    let value = lua_static_number_assignment_value_before_offset_from_query(
                        static_source.source,
                        value,
                        static_source.max_start,
                        lua_unsigned_integer_literal_from_query,
                    )?;
                    value.parse().ok()?
                } else {
                    let value = parse_maybe_quoted_query_text(value)?;
                    value.parse().ok()?
                };
                options.scope_lines = Some(scope_lines);
                parsed_scope_lines = true;
            }
            _ => return None,
        }
        parsed = true;
    }

    parsed.then_some(options)
}

fn quick_select_action_callback_or_key_assignment_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowQuickSelectAction> {
    let value = value.trim();
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_action_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        if let Some(value) = lua_static_wezterm_action_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        ) {
            return quick_select_action_callback_or_key_assignment_from_query(
                Some(static_source),
                &value,
            );
        }
        return quick_select_action_callback_or_key_assignment_from_query(None, value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_wezterm_action_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return quick_select_action_callback_or_key_assignment_from_query(
            Some(static_source),
            &value,
        );
    }
    if let Some(action) = quick_select_selected_text_action_callback_from_query(value) {
        return Some(action);
    }
    if let Some(command) =
        lua_action_callback_perform_action_command_with_static_source(static_source, value)
        && let Some(action) = quick_select_action_from_window_command(command)
    {
        return Some(action);
    }
    if lua_action_callback_from_query_with_static_source(static_source, value) {
        return Some(WindowQuickSelectAction::Nop);
    }
    let action = parse_maybe_quoted_query_text(value)?;
    quick_select_action_from_value(&action)
}

fn quick_select_selected_text_action_callback_from_query(
    value: &str,
) -> Option<WindowQuickSelectAction> {
    let callback = strip_lua_function_call_from_query(value, "wezterm.action_callback")
        .or_else(|| strip_lua_function_call_from_query(value, "action_callback"))?;
    let (body, window_param, pane_param, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(callback)?;
    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    let mut selected_text_variables = Vec::new();
    for (index, start) in starts.iter().copied().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(body.len());
        let statement = lua_trim_start_comments(body.get(start..end)?)?;
        if let Some(name) = quick_select_selected_text_local_name_from_callback_statement(
            statement,
            window_param,
            pane_param,
        )? {
            selected_text_variables.push(name.to_owned());
            continue;
        }
        if let Some(action) = quick_select_open_selected_text_action_from_callback_statement(
            statement,
            window_param,
            pane_param,
            &selected_text_variables,
        )? {
            return Some(action);
        }
        if let Some(action) = quick_select_selected_text_action_from_callback_statement(
            statement,
            window_param,
            pane_param,
            &selected_text_variables,
        )? {
            return Some(action);
        }
    }
    None
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn quick_select_selected_text_local_name_from_callback_statement<'a>(
    statement: &'a str,
    window_param: &str,
    pane_param: &str,
) -> Option<Option<&'a str>> {
    let statement = lua_trim_start_comments(statement)?;
    let Some(rest) = statement.strip_prefix("local") else {
        return Some(None);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(None);
    }
    let rest = lua_trim_start_comments(rest)?;
    let name = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(name.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    let expression = lua_trim_end_statement_separator(rest).trim();
    if quick_select_callback_argument_is_selected_text(expression, window_param, pane_param)? {
        return Some(Some(name));
    }
    Some(None)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn quick_select_open_selected_text_action_from_callback_statement(
    statement: &str,
    window_param: &str,
    pane_param: &str,
    selected_text_variables: &[String],
) -> Option<Option<WindowQuickSelectAction>> {
    let statement = lua_trim_start_comments(statement)?;
    let Some(rest) = statement.strip_prefix("wezterm") else {
        return Some(None);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(None);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("open_with")
        || !lua_config_assignment_field_has_boundaries(rest, 0, "open_with")
    {
        return Some(None);
    }
    let rest = lua_trim_start_comments(rest.get("open_with".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [argument] = arguments.as_slice() else {
        return Some(None);
    };
    if !quick_select_callback_argument_is_selected_text_source(
        argument.trim(),
        window_param,
        pane_param,
        selected_text_variables,
    )? || !lua_trim_end_statement_separator(rest).trim().is_empty()
    {
        return Some(None);
    }
    Some(Some(WindowQuickSelectAction::OpenUri))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn quick_select_selected_text_action_from_callback_statement(
    statement: &str,
    window_param: &str,
    pane_param: &str,
    selected_text_variables: &[String],
) -> Option<Option<WindowQuickSelectAction>> {
    let statement = lua_trim_start_comments(statement)?;
    let Some(rest) = statement.strip_prefix(pane_param) else {
        return Some(None);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(None);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let (command_name, action) = if rest.starts_with("send_text")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_text")
    {
        ("send_text", WindowQuickSelectAction::SendSelectedText)
    } else if rest.starts_with("send_paste")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_paste")
    {
        ("send_paste", WindowQuickSelectAction::PasteSelectedText)
    } else {
        return Some(None);
    };
    let rest = lua_trim_start_comments(rest.get(command_name.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [argument] = arguments.as_slice() else {
        return Some(None);
    };
    if !quick_select_callback_argument_is_selected_text_source(
        argument.trim(),
        window_param,
        pane_param,
        selected_text_variables,
    )? || !lua_trim_end_statement_separator(rest).trim().is_empty()
    {
        return Some(None);
    }
    Some(Some(action))
}

fn quick_select_callback_argument_is_selected_text_source(
    argument: &str,
    window_param: &str,
    pane_param: &str,
    selected_text_variables: &[String],
) -> Option<bool> {
    if quick_select_callback_argument_is_selected_text(argument, window_param, pane_param)? {
        return Some(true);
    }
    let argument = lua_trim_start_comments(argument)?;
    let Some(name) = lua_identifier_literal_from_query(argument) else {
        return Some(false);
    };
    Some(
        selected_text_variables
            .iter()
            .any(|variable| variable == name)
            && lua_static_identifier_value_rest_is_statement_end(argument.get(name.len()..)?),
    )
}

fn quick_select_callback_argument_is_selected_text(
    argument: &str,
    window_param: &str,
    pane_param: &str,
) -> Option<bool> {
    let argument = lua_trim_start_comments(argument)?;
    let Some(rest) = argument.strip_prefix(window_param) else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("get_selection_text_for_pane")
        || !lua_config_assignment_field_has_boundaries(rest, 0, "get_selection_text_for_pane")
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest.get("get_selection_text_for_pane".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [pane] = arguments.as_slice() else {
        return Some(false);
    };
    let pane = lua_trim_start_comments(pane.trim())?;
    let name = lua_identifier_literal_from_query(pane)?;
    Some(
        name == pane_param
            && lua_static_identifier_value_rest_is_statement_end(pane.get(name.len()..)?)
            && rest.trim().is_empty(),
    )
}

fn normalized_quick_select_lua_field(field: &str) -> String {
    field
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn pane_select_options_from_query(query: &str) -> Option<WindowPaneSelectOptions> {
    pane_select_options_from_query_with_static_source(None, query)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn pane_select_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowPaneSelectOptions> {
    if let Some(options) = pane_select_lua_table_from_query_with_static_source(static_source, query)
    {
        return Some(options);
    }

    if static_source.is_some() {
        return pane_select_options_from_query_with_static_source(None, query);
    }

    let rest = strip_query_prefix_from_any(
        query,
        &["pane select=", "pane select ", "paneselect=", "paneselect "],
    )?;
    let tokens = command_palette_query_words(rest)?;
    let mut options = WindowPaneSelectOptions {
        mode: WindowPaneSelectMode::Activate,
        show_pane_ids: false,
        alphabet: None,
    };
    let mut parsed_mode = false;
    let mut parsed_show_pane_ids = false;
    let mut parsed_alphabet = false;
    let mut index = 0;
    while index < tokens.len() {
        if let Some(value) = query_assignment_value_from_token(tokens[index].as_str(), &["mode"]) {
            if parsed_mode {
                return None;
            }
            options.mode = pane_select_mode_option_from_query(value)?;
            parsed_mode = true;
            index += 1;
            continue;
        }
        if let Some(value) = query_assignment_value_from_token(
            tokens[index].as_str(),
            &["show_pane_ids", "show-pane-ids"],
        ) {
            if parsed_show_pane_ids {
                return None;
            }
            options.show_pane_ids = bool_from_query(value)?;
            parsed_show_pane_ids = true;
            index += 1;
            continue;
        }
        if let Some(value) =
            query_assignment_value_from_token(tokens[index].as_str(), &["alphabet"])
        {
            if parsed_alphabet {
                return None;
            }
            options.alphabet = Some(parse_non_empty_query_text(value)?.to_owned());
            parsed_alphabet = true;
            index += 1;
            continue;
        }
        let token_key = tokens[index].to_ascii_lowercase();
        match token_key.as_str() {
            "mode" => {
                if parsed_mode {
                    return None;
                }
                options.mode = pane_select_mode_option_from_query(tokens.get(index + 1)?.as_str())?;
                parsed_mode = true;
                index += 2;
            }
            "alphabet" => {
                if parsed_alphabet {
                    return None;
                }
                let alphabet = parse_non_empty_query_text(tokens.get(index + 1)?.as_str())?;
                options.alphabet = Some(alphabet.to_owned());
                parsed_alphabet = true;
                index += 2;
            }
            "show_pane_ids" | "show-pane-ids" => {
                if parsed_show_pane_ids {
                    return None;
                }
                options.show_pane_ids =
                    bool_from_query(parse_single_query_value(tokens.get(index + 1)?.as_str())?)?;
                parsed_show_pane_ids = true;
                index += 2;
            }
            "show"
                if tokens
                    .get(index + 1)
                    .is_some_and(|token| token.eq_ignore_ascii_case("pane"))
                    && tokens.get(index + 2).is_some_and(|token| {
                        token.eq_ignore_ascii_case("ids")
                            || starts_with_ascii_case_insensitive(token, "ids=")
                    }) =>
            {
                if parsed_show_pane_ids {
                    return None;
                }
                if let Some(value) =
                    query_assignment_value_from_token(tokens.get(index + 2)?.as_str(), &["ids"])
                {
                    options.show_pane_ids = bool_from_query(value)?;
                    index += 3;
                } else {
                    options.show_pane_ids = bool_from_query(parse_single_query_value(
                        tokens.get(index + 3)?.as_str(),
                    )?)?;
                    index += 4;
                }
                parsed_show_pane_ids = true;
            }
            _ => return None,
        }
    }
    parsed_mode.then_some(options)
}

fn pane_select_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowPaneSelectOptions> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    let value = strip_lua_function_call_from_query(query, "paneselect")
        .or_else(|| strip_query_table_assignment_from_prefix(query, "paneselect="))?;
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut options = WindowPaneSelectOptions {
        mode: WindowPaneSelectMode::Activate,
        show_pane_ids: false,
        alphabet: None,
    };
    let mut parsed = false;
    let mut parsed_mode = false;
    let mut parsed_show_pane_ids = false;
    let mut parsed_alphabet = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();

        match normalized_pane_select_lua_field(&name).as_str() {
            "mode" => {
                if parsed_mode {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                options.mode = pane_select_mode_option_from_query(&value)?;
                parsed_mode = true;
            }
            "showpaneids" => {
                if parsed_show_pane_ids {
                    return None;
                }
                options.show_pane_ids = parse_maybe_static_query_bool(static_source, value)?;
                parsed_show_pane_ids = true;
            }
            "alphabet" => {
                if parsed_alphabet {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                if value.is_empty() {
                    return None;
                }
                options.alphabet = Some(value);
                parsed_alphabet = true;
            }
            _ => return None,
        }
        parsed = true;
    }

    parsed.then_some(options)
}

fn normalized_pane_select_lua_field(field: &str) -> String {
    field
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn pane_select_mode_option_from_query(mode: &str) -> Option<WindowPaneSelectMode> {
    let normalized = mode
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "activate" => Some(WindowPaneSelectMode::Activate),
        "swap" | "swapwithactive" => Some(WindowPaneSelectMode::SwapWithActive),
        "swapkeepfocus" | "swapwithactivekeepfocus" => {
            Some(WindowPaneSelectMode::SwapWithActiveKeepFocus)
        }
        "movetonewtab" => Some(WindowPaneSelectMode::MoveToNewTab),
        "movetonewwindow" => Some(WindowPaneSelectMode::MoveToNewWindow),
        _ => None,
    }
}

fn pane_select_alphabet_from_query(query: &str) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "pane select alphabet ",
            "pane select alphabet=",
            "paneselect alphabet ",
            "paneselect alphabet=",
        ],
    )
    .and_then(parse_maybe_quoted_query_text)
}

fn pane_select_show_pane_ids_alphabet_from_query(query: &str) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "pane select show pane ids alphabet ",
            "pane select show pane ids alphabet=",
            "pane select show-pane-ids alphabet ",
            "pane select show-pane-ids alphabet=",
            "paneselect show pane ids alphabet ",
            "paneselect show pane ids alphabet=",
            "paneselect show-pane-ids alphabet ",
            "paneselect show-pane-ids alphabet=",
        ],
    )
    .and_then(parse_maybe_quoted_query_text)
}

fn pane_select_activate_alphabet_from_query(query: &str) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "pane select activate alphabet ",
            "pane select activate alphabet=",
            "paneselect activate alphabet ",
            "paneselect activate alphabet=",
        ],
    )
    .and_then(parse_maybe_quoted_query_text)
}

fn pane_select_activate_show_pane_ids_alphabet_from_query(query: &str) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "pane select activate show pane ids alphabet ",
            "pane select activate show pane ids alphabet=",
            "pane select activate show-pane-ids alphabet ",
            "pane select activate show-pane-ids alphabet=",
            "paneselect activate show pane ids alphabet ",
            "paneselect activate show pane ids alphabet=",
            "paneselect activate show-pane-ids alphabet ",
            "paneselect activate show-pane-ids alphabet=",
        ],
    )
    .and_then(parse_maybe_quoted_query_text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowPaneSelectModeAlphabetQuery {
    command: WindowCommand,
    mode: WindowPaneSelectMode,
    alphabet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowPaneSelectModeShowPaneIdsQuery {
    command: WindowCommand,
    mode: WindowPaneSelectMode,
    alphabet: Option<String>,
}

fn pane_select_mode_alphabet_from_query(query: &str) -> Option<WindowPaneSelectModeAlphabetQuery> {
    let query = query.trim();
    [
        (
            "pane select swap keep focus alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "pane select swap keep focus alphabet=",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "paneselect swap keep focus alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "paneselect swap keep focus alphabet=",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "pane select swap alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "pane select swap alphabet=",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "paneselect swap alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "paneselect swap alphabet=",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "pane select move to new tab alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "pane select move to new tab alphabet=",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "paneselect move to new tab alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "paneselect move to new tab alphabet=",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "pane select move to new window alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "pane select move to new window alphabet=",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "paneselect move to new window alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "paneselect move to new window alphabet=",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
    ]
    .into_iter()
    .find_map(|(prefix, command, mode)| {
        strip_query_prefix_from_any(query, &[prefix])
            .and_then(parse_maybe_quoted_query_text)
            .map(|alphabet| WindowPaneSelectModeAlphabetQuery {
                command,
                mode,
                alphabet,
            })
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn pane_select_mode_show_pane_ids_from_query(
    query: &str,
) -> Option<WindowPaneSelectModeShowPaneIdsQuery> {
    let query = query.trim();
    [
        (
            "pane select swap keep focus show pane ids",
            "pane select swap keep focus show pane ids alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "pane select swap keep focus show_pane_ids",
            "pane select swap keep focus show_pane_ids alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "pane select swap keep focus show-pane-ids",
            "pane select swap keep focus show-pane-ids alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "paneselect swap keep focus show pane ids",
            "paneselect swap keep focus show pane ids alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "paneselect swap keep focus show_pane_ids",
            "paneselect swap keep focus show_pane_ids alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "paneselect swap keep focus show-pane-ids",
            "paneselect swap keep focus show-pane-ids alphabet ",
            WindowCommand::EnterPaneSwapKeepFocus,
            WindowPaneSelectMode::SwapWithActiveKeepFocus,
        ),
        (
            "pane select swap show pane ids",
            "pane select swap show pane ids alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "pane select swap show_pane_ids",
            "pane select swap show_pane_ids alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "pane select swap show-pane-ids",
            "pane select swap show-pane-ids alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "paneselect swap show pane ids",
            "paneselect swap show pane ids alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "paneselect swap show_pane_ids",
            "paneselect swap show_pane_ids alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "paneselect swap show-pane-ids",
            "paneselect swap show-pane-ids alphabet ",
            WindowCommand::EnterPaneSwap,
            WindowPaneSelectMode::SwapWithActive,
        ),
        (
            "pane select move to new tab show pane ids",
            "pane select move to new tab show pane ids alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "pane select move to new tab show_pane_ids",
            "pane select move to new tab show_pane_ids alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "pane select move to new tab show-pane-ids",
            "pane select move to new tab show-pane-ids alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "paneselect move to new tab show pane ids",
            "paneselect move to new tab show pane ids alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "paneselect move to new tab show_pane_ids",
            "paneselect move to new tab show_pane_ids alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "paneselect move to new tab show-pane-ids",
            "paneselect move to new tab show-pane-ids alphabet ",
            WindowCommand::EnterPaneMoveToNewTab,
            WindowPaneSelectMode::MoveToNewTab,
        ),
        (
            "pane select move to new window show pane ids",
            "pane select move to new window show pane ids alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "pane select move to new window show_pane_ids",
            "pane select move to new window show_pane_ids alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "pane select move to new window show-pane-ids",
            "pane select move to new window show-pane-ids alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "paneselect move to new window show pane ids",
            "paneselect move to new window show pane ids alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "paneselect move to new window show_pane_ids",
            "paneselect move to new window show_pane_ids alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
        (
            "paneselect move to new window show-pane-ids",
            "paneselect move to new window show-pane-ids alphabet ",
            WindowCommand::EnterPaneMoveToNewWindow,
            WindowPaneSelectMode::MoveToNewWindow,
        ),
    ]
    .into_iter()
    .find_map(|(plain_query, alphabet_prefix, command, mode)| {
        if query.eq_ignore_ascii_case(plain_query) {
            return Some(WindowPaneSelectModeShowPaneIdsQuery {
                command,
                mode,
                alphabet: None,
            });
        }

        let alphabet =
            if let Some(alphabet) = strip_query_prefix_from_any(query, &[alphabet_prefix]) {
                parse_maybe_quoted_query_text(alphabet)
            } else {
                let assignment_prefix = alphabet_prefix.strip_suffix(' ').map(|prefix| {
                    let mut prefix = prefix.to_owned();
                    prefix.push('=');
                    prefix
                })?;
                strip_query_prefix_from_any(query, &[assignment_prefix.as_str()])
                    .and_then(parse_maybe_quoted_query_text)
            }?;

        Some(WindowPaneSelectModeShowPaneIdsQuery {
            command,
            mode,
            alphabet: Some(alphabet),
        })
    })
}

fn spawn_command_query_from_prefix(query: &str, prefix: &str) -> Option<WindowSpawnCommandQuery> {
    let command = strip_query_prefix_from_any(query, &[prefix])?;
    if command.trim_start().starts_with('{') {
        return None;
    }
    let words = command_palette_query_words(command)?;
    let mut words = words.iter().map(String::as_str).peekable();
    let mut options = parse_spawn_command_query_options(&mut words).ok()?;
    loop {
        if let Some(value) = words
            .peek()
            .and_then(|word| query_text_assignment_value_from_token(word, &["cwd"]))
        {
            options.cwd = Some(non_empty_spawn_command_option_value(value).ok()?);
            words.next();
            continue;
        }
        if let Some(value) = words.peek().and_then(|word| {
            query_text_assignment_value_from_token(
                word,
                &["set_environment_variables", "set-environment-variables"],
            )
        }) {
            let (name, value) = spawn_command_environment_from_query(value).ok()?;
            options.environment.insert(name, value);
            words.next();
            continue;
        }
        if let Some(value) = words
            .peek()
            .and_then(|word| query_text_assignment_value_from_token(word, &["domain"]))
        {
            options.domain = Some(spawn_command_domain_from_query(value)?);
            words.next();
            continue;
        }
        if let Some(value) = words
            .peek()
            .and_then(|word| query_text_assignment_value_from_token(word, &["position"]))
        {
            options.window_position = Some(spawn_command_window_position_from_query(value).ok()?);
            words.next();
            continue;
        }
        break;
    }
    let program = words.next()?;
    let program = query_assignment_value_from_token(program, &["args"])
        .unwrap_or(program)
        .to_owned();
    let args = words.map(str::to_owned).collect::<Vec<_>>();

    Some(WindowSpawnCommandQuery {
        label: None,
        program,
        args,
        cwd: options.cwd,
        environment: options.environment,
        domain: options.domain,
        window_position: options.window_position,
    })
}

fn command_palette_query_words(query: &str) -> Option<Vec<String>> {
    command_palette_query_word_spans(query)
        .map(|words| words.into_iter().map(|(word, _, _)| word).collect())
}

fn command_palette_query_word_spans(query: &str) -> Option<Vec<(String, usize, usize)>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut in_word = false;
    let mut word_start = 0;

    for (index, character) in query.char_indices() {
        if escaped {
            word.push(character);
            escaped = false;
            in_word = true;
            continue;
        }

        match quote {
            Some(_) if character == '\\' => {
                escaped = true;
                in_word = true;
            }
            Some(active_quote) if character == active_quote => {
                quote = None;
                in_word = true;
            }
            None if character == '"' || character == '\'' => {
                if !in_word {
                    word_start = index;
                }
                quote = Some(character);
                in_word = true;
            }
            None if character.is_whitespace() => {
                if in_word {
                    words.push((std::mem::take(&mut word), word_start, index));
                    in_word = false;
                }
            }
            Some(_) | None => {
                if !in_word {
                    word_start = index;
                }
                word.push(character);
                in_word = true;
            }
        }
    }

    if quote.is_some() {
        return None;
    }

    if escaped {
        word.push('\\');
    }

    if in_word {
        words.push((word, word_start, query.len()));
    }

    Some(words)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowSpawnCommandQueryOptions {
    cwd: Option<String>,
    environment: BTreeMap<String, String>,
    domain: Option<WindowSpawnTabDomain>,
    window_position: Option<WindowPosition>,
}

impl WindowSpawnCommandQueryOptions {
    #[expect(
        clippy::unused_self,
        reason = "method shape is retained for compatibility call-site consistency"
    )]
    fn launch_menu_label(&self) -> String {
        "New Tab".to_owned()
    }
}

fn parse_spawn_command_query_options<'a, I>(
    words: &mut std::iter::Peekable<I>,
) -> Result<WindowSpawnCommandQueryOptions, ()>
where
    I: Iterator<Item = &'a str>,
{
    let mut options = WindowSpawnCommandQueryOptions::default();
    while parse_spawn_command_query_option(words, &mut options)? {}
    Ok(options)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn parse_spawn_command_query_option<'a, I>(
    words: &mut std::iter::Peekable<I>,
    options: &mut WindowSpawnCommandQueryOptions,
) -> Result<bool, ()>
where
    I: Iterator<Item = &'a str>,
{
    match words.peek().copied() {
        Some(option) if option.eq_ignore_ascii_case("--domain") => {
            words.next();
            let value = words.next().ok_or(())?;
            options.domain = Some(spawn_command_domain_from_query(value).ok_or(())?);
            Ok(true)
        }
        Some(option) if starts_with_ascii_case_insensitive(option, "--domain=") => {
            let value = strip_query_prefix_from_any(option, &["--domain="]).ok_or(())?;
            words.next();
            options.domain = Some(spawn_command_domain_from_query(value).ok_or(())?);
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("--cwd") => {
            words.next();
            let value = words.next().ok_or(())?;
            options.cwd = Some(non_empty_spawn_command_option_value(value)?);
            Ok(true)
        }
        Some(option) if starts_with_ascii_case_insensitive(option, "--cwd=") => {
            let value = strip_query_prefix_from_any(option, &["--cwd="]).ok_or(())?;
            words.next();
            options.cwd = Some(non_empty_spawn_command_option_value(value)?);
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("--env") => {
            words.next();
            let value = words.next().ok_or(())?;
            let (name, value) = spawn_command_environment_from_query(value)?;
            options.environment.insert(name, value);
            Ok(true)
        }
        Some(option) if starts_with_ascii_case_insensitive(option, "--env=") => {
            let value = strip_query_prefix_from_any(option, &["--env="]).ok_or(())?;
            words.next();
            let (name, value) = spawn_command_environment_from_query(value)?;
            options.environment.insert(name, value);
            Ok(true)
        }
        Some(option)
            if option.eq_ignore_ascii_case("--set-environment-variables")
                || option.eq_ignore_ascii_case("--set_environment_variables") =>
        {
            words.next();
            let value = words.next().ok_or(())?;
            let (name, value) = spawn_command_environment_from_query(value)?;
            options.environment.insert(name, value);
            Ok(true)
        }
        Some(option)
            if starts_with_ascii_case_insensitive(option, "--set-environment-variables=")
                || starts_with_ascii_case_insensitive(option, "--set_environment_variables=") =>
        {
            let value = strip_query_prefix_from_any(
                option,
                &[
                    "--set-environment-variables=",
                    "--set_environment_variables=",
                ],
            )
            .ok_or(())?;
            words.next();
            let (name, value) = spawn_command_environment_from_query(value)?;
            options.environment.insert(name, value);
            Ok(true)
        }
        Some(option) if option.eq_ignore_ascii_case("--position") => {
            words.next();
            let value = words.next().ok_or(())?;
            options.window_position = Some(spawn_command_window_position_from_query(value)?);
            Ok(true)
        }
        Some(option) if starts_with_ascii_case_insensitive(option, "--position=") => {
            let value = strip_query_prefix_from_any(option, &["--position="]).ok_or(())?;
            words.next();
            options.window_position = Some(spawn_command_window_position_from_query(value)?);
            Ok(true)
        }
        Some(option) => {
            if let Some(value) = query_text_assignment_value_from_token(option, &["cwd"]) {
                words.next();
                options.cwd = Some(non_empty_spawn_command_option_value(value)?);
                return Ok(true);
            }
            if let Some(value) = query_text_assignment_value_from_token(option, &["domain"]) {
                words.next();
                options.domain = Some(spawn_command_domain_from_query(value).ok_or(())?);
                return Ok(true);
            }
            if let Some(value) = query_text_assignment_value_from_token(
                option,
                &["set_environment_variables", "set-environment-variables"],
            ) {
                words.next();
                let (name, value) = spawn_command_environment_from_query(value)?;
                options.environment.insert(name, value);
                return Ok(true);
            }
            if let Some(value) = query_text_assignment_value_from_token(option, &["position"]) {
                words.next();
                options.window_position = Some(spawn_command_window_position_from_query(value)?);
                return Ok(true);
            }
            Ok(false)
        }
        None => Ok(false),
    }
}

fn spawn_command_domain_from_query(domain: &str) -> Option<WindowSpawnTabDomain> {
    let domain = domain.trim();
    if domain.is_empty() {
        return None;
    }
    if domain.starts_with('{') {
        return spawn_tab_domain_lua_table_from_query(domain);
    }
    let normalized = domain
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "currentpanedomain" | "currentpane" | "current" => {
            Some(WindowSpawnTabDomain::CurrentPaneDomain)
        }
        "defaultdomain" | "default" => Some(WindowSpawnTabDomain::DefaultDomain),
        _ => Some(WindowSpawnTabDomain::DomainName(domain.to_owned())),
    }
}

fn native_spawn_domain_config_text(domain: &WindowSpawnTabDomain) -> String {
    match domain {
        WindowSpawnTabDomain::CurrentPaneDomain => "CurrentPaneDomain".to_owned(),
        WindowSpawnTabDomain::DefaultDomain => "DefaultDomain".to_owned(),
        WindowSpawnTabDomain::DomainName(name) => name.clone(),
        WindowSpawnTabDomain::DomainId(id) => format!("DomainId:{id}"),
    }
}

fn attach_domain_from_query(query: &str) -> Option<String> {
    attach_domain_from_query_with_static_source(None, query)
}

fn attach_domain_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<String> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(domain) = strip_lua_function_call_from_query(query, "attachdomain") {
        if (domain.trim_start().starts_with('{') || static_source.is_some())
            && let Some(domain) =
                attach_domain_lua_table_from_query_with_static_source(static_source, domain)
        {
            return Some(domain);
        }
        return named_domain_from_query_with_static_source(static_source, domain);
    }

    if let Some(domain) = strip_query_table_assignment_from_prefix(query, "attachdomain=")
        && (domain.trim_start().starts_with('{') || static_source.is_some())
    {
        return attach_domain_lua_table_from_query_with_static_source(static_source, domain);
    }

    let domain = strip_query_prefix_from_any(
        query,
        &[
            "attach domain=",
            "attach domain ",
            "attachdomain=",
            "attachdomain ",
        ],
    )?;
    if domain.trim_start().starts_with('{') {
        return attach_domain_lua_table_from_query_with_static_source(static_source, domain);
    }
    named_domain_from_query_with_static_source(static_source, domain)
}

fn attach_domain_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        let static_source = static_source?;
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )?;
        resolved_value.as_str()
    };
    let table = value.strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut domain_name = None;
    let mut domain_id = None;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if domain_name.is_some() || domain_id.is_some() {
            return None;
        }
        if key.eq_ignore_ascii_case("domainname") {
            let value = parse_maybe_static_query_text(static_source, value)?;
            if value.is_empty() {
                return None;
            }
            domain_name = Some(value);
        } else if key.eq_ignore_ascii_case("domainid") {
            domain_id = Some(parse_maybe_static_usize_query(static_source, value)?);
        } else {
            return None;
        }
    }
    if let Some(domain_name) = domain_name {
        return Some(domain_name);
    }
    domain_id.map(|domain_id| format!("domainid:{domain_id}"))
}
