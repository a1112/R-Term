fn lua_static_wezterm_status_update_event_from_query(
    source: &str,
) -> Option<NativeLuaWindowStatusUpdate> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(update) = lua_static_wezterm_status_update_event_from_statement(source, start) {
            selected = Some(update);
        }
    }
    selected
}
fn lua_static_wezterm_status_update_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaWindowStatusUpdate> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if !matches!(event_name.as_str(), "update-status" | "update-right-status") {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_name, pane_name, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    lua_static_status_update_from_function_body(
        body,
        window_name,
        pane_name,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_status_update_config_overrides_from_query(
    source: &str,
) -> Option<NativeWindowConfigPatch> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(overrides) =
            lua_static_wezterm_status_update_config_overrides_from_statement(source, start)
        {
            selected = Some(overrides);
        }
    }
    selected
}

fn lua_static_wezterm_status_update_config_overrides_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeWindowConfigPatch> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if !matches!(event_name.as_str(), "update-status" | "update-right-status") {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_name, _, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    lua_static_window_config_overrides_from_function_body(
        body,
        window_name,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_bell_event_from_query(source: &str) -> Option<NativeLuaWindowStatusUpdate> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(update) = lua_static_wezterm_bell_event_from_statement(source, start) {
            selected = Some(update);
        }
    }
    selected
}

fn lua_static_wezterm_bell_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaWindowStatusUpdate> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "bell" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_name, pane_name, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    lua_static_status_update_from_function_body(
        body,
        window_name,
        pane_name,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_focus_changed_event_from_query(
    source: &str,
) -> Option<NativeLuaWindowStatusUpdate> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(update) = lua_static_wezterm_focus_changed_event_from_statement(source, start) {
            selected = Some(update);
        }
    }
    selected
}

fn lua_static_wezterm_focus_changed_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaWindowStatusUpdate> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "window-focus-changed" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_name, pane_name, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    lua_static_status_update_from_function_body(
        body,
        window_name,
        pane_name,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_resized_event_from_query(
    source: &str,
) -> Option<NativeLuaWindowStatusUpdate> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(update) = lua_static_wezterm_resized_event_from_statement(source, start) {
            selected = Some(update);
        }
    }
    selected
}

fn lua_static_wezterm_resized_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaWindowStatusUpdate> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "window-resized" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_name, pane_name, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    lua_static_status_update_from_function_body(
        body,
        window_name,
        pane_name,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_config_reloaded_event_from_query(
    source: &str,
) -> Option<NativeLuaWindowStatusUpdate> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(update) = lua_static_wezterm_config_reloaded_event_from_statement(source, start)
        {
            selected = Some(update);
        }
    }
    selected
}

fn lua_static_wezterm_config_reloaded_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaWindowStatusUpdate> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "window-config-reloaded" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_name, pane_name, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    lua_static_status_update_from_function_body(
        body,
        window_name,
        pane_name,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_user_var_changed_event_from_query(
    source: &str,
) -> Option<NativeLuaUserVarChanged> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(update) =
            lua_static_wezterm_user_var_changed_event_from_statement(source, start)
        {
            selected = Some(update);
        }
    }
    selected
}

fn lua_static_wezterm_user_var_changed_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaUserVarChanged> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "user-var-changed" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_name, pane_name, name_param, value_param) =
        lua_anonymous_function_body_and_first_four_params_from_query(callback.as_ref())?;
    lua_static_user_var_changed_from_function_body(
        body,
        window_name,
        pane_name,
        name_param,
        value_param,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_window_title_return_event_from_query(
    source: &str,
) -> Option<NativeLuaWindowTitle> {
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(value) =
            lua_static_wezterm_window_title_return_event_from_statement(source, start)
        {
            return Some(value);
        }
    }
    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_wezterm_window_title_return_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaWindowTitle> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "format-window-title" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, tab_param, pane_param, tabs_param, panes_param) =
        lua_anonymous_function_body_and_first_two_and_optional_third_and_fourth_params_from_query(
            callback.as_ref(),
        )?;
    let tabs_param = tabs_param.unwrap_or("tabs");
    let panes_param = panes_param.unwrap_or("panes");
    lua_static_window_title_return_from_function_body(
        body,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_tab_title_return_event_from_query(source: &str) -> Option<NativeLuaTabTitle> {
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(value) = lua_static_wezterm_tab_title_return_event_from_statement(source, start)
        {
            return Some(value);
        }
    }
    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_wezterm_tab_title_return_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaTabTitle> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "format-tab-title" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, tab_param, tabs_param, panes_param, hover_param) =
        lua_anonymous_function_body_and_format_tab_title_params_from_query(callback.as_ref())?;
    lua_static_tab_title_return_from_function_body(
        body,
        tab_param,
        tabs_param,
        panes_param,
        hover_param,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_open_uri_event_from_query(source: &str) -> Option<NativeLuaOpenUri> {
    let mut handlers = Vec::new();
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(value) = lua_static_wezterm_open_uri_event_from_statement(source, start) {
            handlers.push(value);
        }
    }
    match handlers.len() {
        0 => None,
        1 => handlers.pop(),
        _ => Some(NativeLuaOpenUri::Sequence(handlers)),
    }
}

fn lua_static_wezterm_open_uri_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaOpenUri> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "open-uri" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_param, pane_param, uri_param) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    let uri_param = uri_param.unwrap_or("uri");
    lua_static_open_uri_return_from_function_body(
        body,
        window_param,
        pane_param,
        uri_param,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_new_tab_button_click_event_from_query(
    source: &str,
) -> Option<NativeLuaNewTabButtonClick> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(value) =
            lua_static_wezterm_new_tab_button_click_event_from_statement(source, start)
        {
            selected = Some(value);
        }
    }
    selected
}

fn lua_static_wezterm_new_tab_button_click_event_from_statement(
    source: &str,
    start: usize,
) -> Option<NativeLuaNewTabButtonClick> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "new-tab-button-click" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (params, body) = lua_anonymous_function_params_and_body_from_query(callback.as_ref())?;
    let window_param = params
        .first()
        .and_then(|param| lua_function_param_identifier(param));
    let pane_param = params
        .get(1)
        .and_then(|param| lua_function_param_identifier(param));
    let default_action_param = params
        .get(3)
        .and_then(|param| lua_function_param_identifier(param));
    lua_static_new_tab_button_click_return_from_function_body(
        body,
        window_param,
        pane_param,
        params
            .get(2)
            .and_then(|param| lua_function_param_identifier(param)),
        default_action_param,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

fn lua_static_wezterm_augment_command_palette_event_from_query(
    source: &str,
) -> Option<Vec<NativeCommandPaletteEntry>> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        if let Some(value) =
            lua_static_wezterm_augment_command_palette_event_from_statement(source, start)
        {
            selected = Some(value);
        }
    }
    selected
}

fn lua_static_wezterm_augment_command_palette_event_from_statement(
    source: &str,
    start: usize,
) -> Option<Vec<NativeCommandPaletteEntry>> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    if event_name != "augment-command-palette" {
        return None;
    }
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let body = lua_anonymous_function_body_from_query(callback.as_ref())?;
    lua_static_command_palette_entries_return_from_function_body(
        body,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaEmitEventHandler {
    command: Option<WindowCommand>,
    stop_propagation: bool,
}

fn lua_static_wezterm_emit_event_handlers_from_query(
    source: &str,
) -> Option<BTreeMap<String, Vec<NativeLuaEmitEventHandler>>> {
    let mut handlers: BTreeMap<String, Vec<NativeLuaEmitEventHandler>> = BTreeMap::new();
    for start in lua_top_level_statement_start_indices_before_offset(source, source.len())? {
        let Some((event, command)) =
            lua_static_wezterm_emit_event_handler_from_statement(source, start)
        else {
            continue;
        };
        handlers.entry(event).or_default().push(command);
    }
    (!handlers.is_empty()).then_some(handlers)
}

fn lua_static_wezterm_emit_event_handler_from_statement(
    source: &str,
    start: usize,
) -> Option<(String, NativeLuaEmitEventHandler)> {
    let rest = lua_static_wezterm_on_event_args_from_statement(source, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let (event_name, rest) =
        lua_static_wezterm_on_event_name_and_rest_from_args(source, start, rest)?;
    let callback = lua_static_wezterm_on_callback_query_from_rest(source, start, rest)?;
    let (body, window_param, pane_param, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
            callback.as_ref(),
        )?;
    let outer_static_source = LuaStaticSource {
        source,
        max_start: start,
    };
    let command = lua_action_callback_perform_action_command_body(
        Some(outer_static_source),
        body,
        window_param,
        pane_param,
    );
    let stop_propagation =
        lua_static_bool_return_from_function_body(body, Some(outer_static_source)) == Some(false);
    if command.is_none() && !stop_propagation {
        return None;
    }
    Some((
        event_name,
        NativeLuaEmitEventHandler {
            command,
            stop_propagation,
        },
    ))
}

fn lua_static_wezterm_on_event_name_and_rest_from_args<'a>(
    source: &'a str,
    start: usize,
    args: &'a str,
) -> Option<(String, &'a str)> {
    let args = lua_trim_start_comments(args)?;
    let (argument_list, _) = lua_parenthesized_argument_list_prefix_from_query(args)?;
    let arguments = split_lua_top_level_arguments(argument_list)?;
    let event_arg = arguments.first()?;
    let event_start = lua_source_slice_start_offset(argument_list, event_arg)?;
    let event_end = event_start.checked_add(event_arg.len())?;
    let rest = args.get(event_end..)?;
    let event_name = lua_static_string_value_from_expression(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        None,
        event_arg,
    )?;
    Some((event_name, rest))
}

fn lua_static_wezterm_on_callback_query_from_rest<'a>(
    source: &'a str,
    start: usize,
    rest: &'a str,
) -> Option<Cow<'a, str>> {
    let rest = lua_trim_start_comments(rest)?.strip_prefix(',')?;
    let rest = lua_trim_start_comments(rest)?;
    lua_static_callback_query_from_value(source, start, rest)
}

fn lua_static_callback_query_from_value<'a>(
    source: &'a str,
    start: usize,
    value: &'a str,
) -> Option<Cow<'a, str>> {
    let value = lua_trim_start_comments(value)?;
    if lua_source_keyword_at(value, 0, "function") {
        return Some(Cow::Borrowed(value));
    }

    if let Some(rest) = value.strip_prefix('(') {
        let rest = lua_trim_start_comments(rest)?;
        let (inner, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        let rest = lua_trim_start_comments(rest)?;
        if rest.starts_with(')') {
            return lua_static_callback_query_from_expression(source, start, inner);
        }
    }

    lua_static_callback_query_from_expression_with_call_tail(source, start, value, 0)
}

const LUA_STATIC_CALLBACK_ALIAS_RESOLUTION_LIMIT: usize = 8;

fn lua_static_callback_query_from_expression_with_call_tail<'a>(
    source: &'a str,
    start: usize,
    value: &'a str,
    alias_depth: usize,
) -> Option<Cow<'a, str>> {
    let name = lua_identifier_literal_from_query(value)?;
    let rest = lua_trim_start_comments(value.get(name.len()..)?)?;
    if rest.starts_with(')') {
        if let Some(statement) =
            lua_static_named_function_statement_before_offset(source, name, start)
        {
            let (params, body) =
                lua_named_function_params_and_body_from_statement(statement, name)?;
            return Some(Cow::Owned(format!("function({params}){body} end")));
        }

        let assigned_value = lua_static_expression_variable_assignment_before_offset_from_query(
            source, name, start,
        )?;
        if lua_source_keyword_at(assigned_value, 0, "function") {
            return Some(Cow::Borrowed(assigned_value));
        }
        if alias_depth < LUA_STATIC_CALLBACK_ALIAS_RESOLUTION_LIMIT {
            return lua_static_callback_query_from_expression_with_depth(
                source,
                start,
                assigned_value,
                alias_depth + 1,
            );
        }
        return None;
    }

    let (keys, rest) = lua_table_map_field_path_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        rest,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with(')') {
        return None;
    }

    lua_static_callback_query_from_table_field_path(source, start, name, &keys, alias_depth)
}

fn lua_static_callback_query_from_expression<'a>(
    source: &'a str,
    start: usize,
    value: &'a str,
) -> Option<Cow<'a, str>> {
    lua_static_callback_query_from_expression_with_depth(source, start, value, 0)
}

fn lua_static_callback_query_from_expression_with_depth<'a>(
    source: &'a str,
    start: usize,
    value: &'a str,
    alias_depth: usize,
) -> Option<Cow<'a, str>> {
    let value = lua_trim_start_comments(value)?;
    if let Some(rest) = value.strip_prefix('(') {
        let rest = lua_trim_start_comments(rest)?;
        let (inner, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        let rest = lua_trim_start_comments(rest)?;
        if lua_static_identifier_value_rest_is_statement_end(rest) {
            return lua_static_callback_query_from_expression_with_depth(
                source,
                start,
                inner,
                alias_depth,
            );
        }
    }

    if lua_source_keyword_at(value, 0, "function") {
        return Some(Cow::Borrowed(value));
    }

    let name = lua_identifier_literal_from_query(value)?;
    let rest = lua_trim_start_comments(value.get(name.len()..)?)?;
    if lua_static_identifier_value_rest_is_statement_end(rest) {
        if let Some(statement) =
            lua_static_named_function_statement_before_offset(source, name, start)
        {
            let (params, body) =
                lua_named_function_params_and_body_from_statement(statement, name)?;
            return Some(Cow::Owned(format!("function({params}){body} end")));
        }

        let assigned_value = lua_static_expression_variable_assignment_before_offset_from_query(
            source, name, start,
        )?;
        if lua_source_keyword_at(assigned_value, 0, "function") {
            return Some(Cow::Borrowed(assigned_value));
        }
        if alias_depth < LUA_STATIC_CALLBACK_ALIAS_RESOLUTION_LIMIT {
            return lua_static_callback_query_from_expression_with_depth(
                source,
                start,
                assigned_value,
                alias_depth + 1,
            );
        }
        return None;
    }

    let (keys, rest) = lua_table_map_field_path_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        rest,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    lua_static_callback_query_from_table_field_path(source, start, name, &keys, alias_depth)
}

fn lua_static_callback_query_from_table_field_path<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
    keys: &[String],
    alias_depth: usize,
) -> Option<Cow<'a, str>> {
    if let Some(value) = lua_static_table_field_path_assignment_value_before_offset_from_query(
        source, variable, keys, start,
    ) {
        return lua_static_callback_query_from_resolved_expression_value(
            source,
            start,
            value,
            alias_depth,
        );
    }

    lua_static_table_field_path_function_callback_before_offset(source, variable, keys, start)
}

fn lua_static_callback_query_from_resolved_expression_value<'a>(
    source: &'a str,
    start: usize,
    value: &'a str,
    alias_depth: usize,
) -> Option<Cow<'a, str>> {
    if lua_source_keyword_at(value, 0, "function") {
        return Some(Cow::Borrowed(value));
    }
    if alias_depth < LUA_STATIC_CALLBACK_ALIAS_RESOLUTION_LIMIT {
        return lua_static_callback_query_from_expression_with_depth(
            source,
            start,
            value,
            alias_depth + 1,
        );
    }
    None
}

fn lua_static_table_field_path_assignment_value_before_offset_from_query<'a>(
    source: &'a str,
    variable: &str,
    keys: &[String],
    max_start: usize,
) -> Option<&'a str> {
    lua_static_table_field_path_assignment_value_before_offset_with_depth(
        source, variable, keys, max_start, 0,
    )
}

const LUA_STATIC_TABLE_FIELD_ALIAS_RESOLUTION_LIMIT: usize = 8;

fn lua_static_table_field_path_assignment_value_before_offset_with_depth<'a>(
    source: &'a str,
    variable: &str,
    keys: &[String],
    max_start: usize,
    alias_depth: usize,
) -> Option<&'a str> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let rest = if lua_source_keyword_at(source, start, "local") {
            lua_trim_start_comments(source.get(start + "local".len()..)?)?
        } else {
            source.get(start..)?
        };
        if let Some(table) = lua_static_table_variable_assignment_table_from_query(rest, variable) {
            selected = lua_static_table_field_path_value_from_query(
                LuaStaticSource {
                    source,
                    max_start: start,
                },
                table,
                keys,
                alias_depth,
            )?;
            continue;
        }

        let Some(assignment) =
            lua_static_table_variable_field_path_assignment_from_query(source, start, variable)
        else {
            continue;
        };
        if assignment.keys == keys {
            selected = Some(assignment.value);
        } else if keys.starts_with(&assignment.keys) {
            selected = lua_static_table_field_path_value_from_query(
                LuaStaticSource {
                    source,
                    max_start: start,
                },
                assignment.value,
                &keys[assignment.keys.len()..],
                alias_depth,
            )?;
        }
    }

    selected
}

fn lua_static_table_variable_field_path_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableMapFieldPathAssignment<'a>> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let (keys, rest) = lua_table_map_field_path_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        after_variable,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableMapFieldPathAssignment {
        keys,
        value: lua_top_level_statement_value_from_query(rest)?,
    })
}

fn lua_table_map_field_path_from_query_with_static_source<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    query: &'a str,
) -> Option<(Vec<String>, &'a str)> {
    let mut keys = Vec::new();
    let mut rest = query;

    while let Some((key, next_rest)) =
        lua_table_map_field_key_from_query_with_static_source(static_source, rest)
    {
        keys.push(key);
        rest = lua_trim_start_comments(next_rest)?;
    }

    (!keys.is_empty()).then_some((keys, rest))
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_static_table_field_path_value_from_query<'a>(
    static_source: LuaStaticSource<'a>,
    mut value: &'a str,
    keys: &[String],
    alias_depth: usize,
) -> Option<Option<&'a str>> {
    for (index, key) in keys.iter().enumerate() {
        match lua_table_field_value_from_query_with_static_source(Some(static_source), value, key) {
            Some(Some(next_value)) => {
                value = next_value;
            }
            Some(None) => return Some(None),
            None => {
                let Some(variable) = lua_identifier_literal_from_query(value) else {
                    return Some(None);
                };
                let rest = value.get(variable.len()..)?;
                if !lua_static_identifier_value_rest_is_statement_end(rest)
                    || alias_depth >= LUA_STATIC_TABLE_FIELD_ALIAS_RESOLUTION_LIMIT
                {
                    return Some(None);
                }
                return Some(
                    lua_static_table_field_path_assignment_value_before_offset_with_depth(
                        static_source.source,
                        variable,
                        &keys[index..],
                        static_source.max_start,
                        alias_depth + 1,
                    ),
                );
            }
        }
    }
    Some(Some(value))
}

fn lua_static_named_function_statement_before_offset<'a>(
    source: &'a str,
    function_name: &str,
    max_start: usize,
) -> Option<&'a str> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let Some(statement) = lua_top_level_function_statement_from_index(source, start) else {
            continue;
        };
        if lua_named_function_params_and_body_from_statement(statement, function_name).is_some() {
            selected = Some(statement);
        }
    }

    selected
}

fn lua_static_table_field_path_function_callback_before_offset<'a>(
    source: &'a str,
    variable: &str,
    keys: &[String],
    max_start: usize,
) -> Option<Cow<'a, str>> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let Some(statement) = lua_top_level_function_statement_from_index(source, start) else {
            continue;
        };
        let Some((params, body)) =
            lua_table_field_function_params_and_body_from_statement(statement, variable, keys)
        else {
            continue;
        };
        selected = Some(Cow::Owned(format!("function({params}){body} end")));
    }

    selected
}

fn lua_named_function_params_and_body_from_statement<'a>(
    statement: &'a str,
    function_name: &str,
) -> Option<(&'a str, &'a str)> {
    if !lua_source_keyword_at(statement, 0, "function") {
        return None;
    }

    let rest = lua_trim_start_comments(statement.get("function".len()..)?)?;
    let name = lua_identifier_literal_from_query(rest)?;
    if name != function_name {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(name.len()..)?)?.strip_prefix('(')?;
    let (params, body_start) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let body = lua_static_function_body_until_end(body_start)?;
    Some((params, body))
}

fn lua_table_field_function_params_and_body_from_statement<'a>(
    statement: &'a str,
    variable: &str,
    keys: &[String],
) -> Option<(&'a str, &'a str)> {
    if !lua_source_keyword_at(statement, 0, "function") {
        return None;
    }

    let rest = lua_trim_start_comments(statement.get("function".len()..)?)?;
    let name = lua_identifier_literal_from_query(rest)?;
    if name != variable {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(name.len()..)?)?;
    let (parsed_keys, rest) = lua_table_map_field_path_from_query_with_static_source(None, rest)?;
    if parsed_keys != keys {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let (params, body_start) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let body = lua_static_function_body_until_end(body_start)?;
    Some((params, body))
}

fn lua_static_string_value_from_expression(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    lua_static_string_value_from_expression_with_depth(static_source, outer_static_source, value, 0)
}

fn lua_static_string_value_from_expression_with_depth(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    depth: usize,
) -> Option<String> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }
    let value = lua_trim_start_comments(value.trim())?;
    let value = lua_tostring_argument_from_query(value).unwrap_or(value);
    if let Some((literal, literal_len)) = lua_inline_string_literal_value_and_len(value) {
        return lua_trim_start_comments(value.get(literal_len..)?)?
            .is_empty()
            .then_some(literal);
    }

    if let Some(value) = lua_static_wezterm_nerdfonts_value_from_expression(
        static_source,
        outer_static_source,
        value,
    ) {
        return Some(value);
    }

    if value.contains("..") {
        let mut resolved = String::new();
        for segment in split_lua_string_concat_segments(value)? {
            resolved.push_str(&lua_static_string_value_from_expression_with_depth(
                static_source,
                outer_static_source,
                segment,
                depth + 1,
            )?);
        }
        return (!resolved.is_empty()).then_some(resolved);
    }

    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return lua_static_string_value_from_expression_with_depth(
            Some(static_source),
            outer_static_source,
            value,
            depth + 1,
        );
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            outer_static_source.source,
            value,
            outer_static_source.max_start,
        )
    {
        return lua_static_string_value_from_expression_with_depth(
            static_source,
            Some(outer_static_source),
            value,
            depth + 1,
        );
    }

    None
}

fn lua_static_wezterm_nerdfonts_value_from_expression(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    let value = lua_trim_start_comments(value.trim())?;
    let rest = lua_static_wezterm_receiver_rest_from_expression(
        value,
        static_source,
        outer_static_source,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_table_map_field_key_from_query_with_static_sources(
        static_source,
        outer_static_source,
        rest,
    )?;
    if field != "nerdfonts" {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let (name, rest) = lua_table_map_field_key_from_query_with_static_sources(
        static_source,
        outer_static_source,
        rest,
    )?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    let value = match name.as_str() {
        "pl_left_hard_divider" => "\u{e0b0}",
        "pl_right_hard_divider" => "\u{e0b2}",
        _ => return None,
    };
    Some(value.to_owned())
}

fn lua_static_wezterm_receiver_rest_from_expression<'a>(
    expression: &'a str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<&'a str> {
    if let Some(static_source) = static_source
        && let Some(rest) = lua_static_wezterm_receiver_rest_from_query(
            static_source.source,
            static_source.max_start,
            expression,
        )
    {
        return Some(rest);
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(rest) = lua_static_wezterm_receiver_rest_from_query(
            outer_static_source.source,
            outer_static_source.max_start,
            expression,
        )
    {
        return Some(rest);
    }

    let rest = expression.strip_prefix("wezterm")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    Some(rest)
}

fn lua_static_wezterm_on_event_args_from_statement(source: &str, start: usize) -> Option<&str> {
    let statement = lua_trim_start_comments(source.get(start..)?)?;
    if let Some(rest) =
        lua_static_wezterm_on_event_args_from_wezterm_query(source, start, statement)
    {
        return Some(rest);
    }
    if let Some(rest) =
        lua_static_wezterm_on_event_args_from_require_query(source, start, statement)
    {
        return Some(rest);
    }

    let alias = lua_identifier_literal_from_query(statement)?;
    let rest = statement.get(alias.len()..)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    if lua_static_wezterm_module_alias_before_offset(source, alias, start)? {
        return lua_static_wezterm_on_event_args_from_receiver_rest(source, start, rest);
    }
    if !lua_static_wezterm_on_alias_before_offset(source, alias, start)? {
        return None;
    }

    lua_trim_start_comments(rest)?.strip_prefix('(')
}

fn lua_static_wezterm_module_alias_before_offset(
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
        selected = lua_static_wezterm_module_alias_value_from_query(value);
    }

    Some(selected)
}

fn lua_static_wezterm_module_alias_value_from_query(value: &str) -> bool {
    let Some(value) = lua_trim_start_comments(value) else {
        return false;
    };
    if let Some(rest) = value.strip_prefix("wezterm") {
        return !rest.chars().next().is_some_and(is_lua_identifier_character)
            && lua_static_wezterm_module_alias_receiver_rest_is_statement_end(rest);
    }
    let Some(value) = lua_top_level_statement_value_from_query(value) else {
        return false;
    };
    let Some(rest) = value.strip_prefix("require") else {
        return false;
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return false;
    }
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let literal = if let Some(rest) = rest.strip_prefix('(') {
        let Some((arguments, rest)) = lua_parenthesized_argument_list_prefix_from_query(rest)
        else {
            return false;
        };
        if !lua_static_identifier_value_rest_is_statement_end(rest) {
            return false;
        }
        let Some(arguments) = split_lua_top_level_arguments(arguments) else {
            return false;
        };
        let [literal] = arguments.as_slice() else {
            return false;
        };
        *literal
    } else {
        rest
    };
    let Some(literal) = lua_quoted_string_literal_from_query(literal)
        .or_else(|| lua_long_bracket_literal_from_query(literal))
    else {
        return false;
    };
    parse_maybe_quoted_query_text(literal).as_deref() == Some("wezterm")
}

fn lua_static_wezterm_module_alias_receiver_rest_is_statement_end(rest: &str) -> bool {
    let rest = rest.trim_start_matches([' ', '\t', '\r']);
    if rest.is_empty() || rest.starts_with(';') {
        return true;
    }
    if let Some(rest) = rest.strip_prefix('\n') {
        return !lua_static_wezterm_module_alias_receiver_rest_is_accessor_continuation(rest);
    }
    let Some(rest) = rest.strip_prefix("--") else {
        return false;
    };
    if let Some((content_start, closing)) = parse_lua_long_bracket_delimiters(rest) {
        let content_and_rest = &rest[content_start..];
        let Some(close_index) = content_and_rest.find(&closing) else {
            return false;
        };
        return lua_static_wezterm_module_alias_receiver_rest_is_statement_end(
            &content_and_rest[close_index + closing.len()..],
        );
    }
    let Some(newline) = rest.find('\n') else {
        return true;
    };
    !lua_static_wezterm_module_alias_receiver_rest_is_accessor_continuation(
        &rest[newline + '\n'.len_utf8()..],
    )
}

fn lua_static_wezterm_module_alias_receiver_rest_is_accessor_continuation(rest: &str) -> bool {
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    rest.starts_with('.') || rest.starts_with('[')
}

fn lua_static_wezterm_on_event_args_from_wezterm_query<'a>(
    source: &str,
    start: usize,
    value: &'a str,
) -> Option<&'a str> {
    let rest = value.strip_prefix("wezterm")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    lua_static_wezterm_on_event_args_from_receiver_rest(source, start, rest)
}

fn lua_static_wezterm_on_event_args_from_require_query<'a>(
    source: &str,
    start: usize,
    value: &'a str,
) -> Option<&'a str> {
    let rest = lua_static_wezterm_require_receiver_rest_from_query(value)?;
    lua_static_wezterm_on_event_args_from_receiver_rest(source, start, rest)
}

fn lua_static_wezterm_require_receiver_rest_from_query(value: &str) -> Option<&str> {
    let rest = value.strip_prefix("require")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let (literal, rest) = if let Some(rest) = rest.strip_prefix('(') {
        let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        let arguments = split_lua_top_level_arguments(arguments)?;
        let [literal] = arguments.as_slice() else {
            return None;
        };
        (*literal, rest)
    } else {
        let literal = lua_quoted_string_literal_from_query(rest)
            .or_else(|| lua_long_bracket_literal_from_query(rest))?;
        (literal, rest.get(literal.len()..)?)
    };
    let literal = lua_quoted_string_literal_from_query(literal)
        .or_else(|| lua_long_bracket_literal_from_query(literal))?;
    if parse_maybe_quoted_query_text(literal).as_deref() != Some("wezterm") {
        return None;
    }
    Some(rest)
}

fn lua_static_wezterm_receiver_rest_from_query<'a>(
    source: &str,
    max_start: usize,
    value: &'a str,
) -> Option<&'a str> {
    if let Some(value) = lua_trim_start_comments(value)
        && let Some(rest) = value.strip_prefix('(')
        && let Some((receiver, rest)) = lua_parenthesized_argument_list_prefix_from_query(rest)
    {
        let receiver_rest =
            lua_static_wezterm_receiver_rest_from_query(source, max_start, receiver.trim())?;
        if lua_static_wezterm_module_alias_receiver_rest_is_statement_end(receiver_rest) {
            return Some(rest);
        }
    }

    if let Some(rest) = lua_static_wezterm_require_receiver_rest_from_query(value) {
        return Some(rest);
    }

    let receiver = lua_identifier_literal_from_query(value)?;
    let rest = value.get(receiver.len()..)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    if receiver == "wezterm"
        || lua_static_wezterm_module_alias_before_offset(source, receiver, max_start)?
    {
        return Some(rest);
    }
    None
}

fn lua_static_wezterm_on_event_args_from_receiver_rest<'a>(
    source: &str,
    start: usize,
    rest: &'a str,
) -> Option<&'a str> {
    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        rest,
    )?;
    if field != "on" {
        return None;
    }
    lua_trim_start_comments(rest)?.strip_prefix('(')
}

fn lua_static_wezterm_on_alias_before_offset(
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
        selected = lua_static_wezterm_on_alias_value_from_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_on_alias_value_from_query(
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
    if value.starts_with("wezterm") && rest.chars().next().is_some_and(is_lua_identifier_character)
    {
        return false;
    }
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some((field, rest)) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource { source, max_start }),
        rest,
    ) else {
        return false;
    };
    if field != "on" {
        return false;
    }
    lua_static_identifier_value_rest_is_statement_end(rest)
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_anonymous_function_body_and_format_tab_title_params_from_query(
    value: &str,
) -> Option<(&str, &str, &str, &str, Option<&str>)> {
    let (params, body) = lua_anonymous_function_params_and_body_from_query(value)?;
    let tab_param = lua_function_param_identifier(params.first()?)?;
    let tabs_param = params
        .get(1)
        .and_then(|param| lua_function_param_identifier(param))
        .unwrap_or("tabs");
    let panes_param = params
        .get(2)
        .and_then(|param| lua_function_param_identifier(param))
        .unwrap_or("panes");
    let hover_param = params
        .get(4)
        .and_then(|param| lua_function_param_identifier(param));
    Some((body, tab_param, tabs_param, panes_param, hover_param))
}

fn lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(
    value: &str,
) -> Option<(&str, &str, &str, Option<&str>)> {
    let (params, body) = lua_anonymous_function_params_and_body_from_query(value)?;
    let first_param = lua_function_param_identifier(params.first()?)?;
    let second_param = lua_function_param_identifier(params.get(1)?)?;
    let third_param = params
        .get(2)
        .and_then(|param| lua_function_param_identifier(param));
    Some((body, first_param, second_param, third_param))
}

#[expect(
    clippy::type_complexity,
    reason = "tuple shape mirrors the compatibility data contract"
)]
fn lua_anonymous_function_body_and_first_two_and_optional_third_and_fourth_params_from_query(
    value: &str,
) -> Option<(&str, &str, &str, Option<&str>, Option<&str>)> {
    let (params, body) = lua_anonymous_function_params_and_body_from_query(value)?;
    let first_param = lua_function_param_identifier(params.first()?)?;
    let second_param = lua_function_param_identifier(params.get(1)?)?;
    let third_param = params
        .get(2)
        .and_then(|param| lua_function_param_identifier(param));
    let fourth_param = params
        .get(3)
        .and_then(|param| lua_function_param_identifier(param));
    Some((body, first_param, second_param, third_param, fourth_param))
}

fn lua_anonymous_function_body_and_first_four_params_from_query(
    value: &str,
) -> Option<(&str, &str, &str, &str, &str)> {
    let (params, body) = lua_anonymous_function_params_and_body_from_query(value)?;
    let first_param = lua_function_param_identifier(params.first()?)?;
    let second_param = lua_function_param_identifier(params.get(1)?)?;
    let third_param = lua_function_param_identifier(params.get(2)?)?;
    let fourth_param = lua_function_param_identifier(params.get(3)?)?;
    Some((body, first_param, second_param, third_param, fourth_param))
}

fn lua_anonymous_function_body_from_query(value: &str) -> Option<&str> {
    let (_, body) = lua_anonymous_function_params_and_body_from_query(value)?;
    Some(body)
}

fn lua_anonymous_function_params_and_body_from_query(value: &str) -> Option<(Vec<&str>, &str)> {
    let value = lua_trim_start_comments(value)?;
    if !lua_source_keyword_at(value, 0, "function") {
        return None;
    }
    let rest = lua_trim_start_comments(value.get("function".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (params, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let params = split_lua_top_level_arguments(params)?;
    let body = lua_static_function_body_until_end(rest)?;
    Some((params, body))
}

fn lua_function_param_identifier(value: &str) -> Option<&str> {
    let value = lua_trim_start_comments(value.trim())?;
    let value = lua_trim_end_comments(value)?;
    let name = lua_identifier_literal_from_query(value)?;
    (name.len() == value.len()).then_some(name)
}

fn lua_static_function_body_until_end(value: &str) -> Option<&str> {
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;

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

        if value[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&value[index + 2..])
            {
                let content_and_rest = &value[index + 2 + content_start..];
                block_comment_end = Some(
                    content_and_rest
                        .find(&closing)
                        .map_or(value.len(), |close_index| {
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
                parse_lua_long_bracket_delimiters(&value[index..])
        {
            let content_and_rest = &value[index + content_start..];
            long_bracket_end = Some(
                content_and_rest
                    .find(&closing)
                    .map_or(value.len(), |close_index| {
                        index + content_start + close_index + closing.len()
                    }),
            );
            continue;
        }

        if lua_source_keyword_at(value, index, "function")
            || lua_source_keyword_at(value, index, "if")
            || lua_source_keyword_at(value, index, "do")
            || lua_source_keyword_at(value, index, "repeat")
        {
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }

        if lua_source_keyword_at(value, index, "end") {
            if lua_block_depth == 0 {
                return value.get(..index).map(str::trim);
            }
            lua_block_depth = lua_block_depth.saturating_sub(1);
            continue;
        }

        if lua_source_keyword_at(value, index, "until") {
            if lua_block_depth == 0 {
                return None;
            }
            lua_block_depth = lua_block_depth.saturating_sub(1);
        }
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_window_title_return_from_function_body(
    body: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowTitle> {
    if let Some(parts) = lua_window_title_explicit_title_fallback_parts_from_function_body(
        body,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        outer_static_source,
        0,
    ) {
        return Some(NativeLuaWindowTitle::Concat(parts));
    }

    if let Some(value) = lua_static_window_title_conditional_return_from_function_body(
        body,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        outer_static_source,
    ) {
        return Some(value);
    }

    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(value) =
            lua_window_title_event_field_return_from_statement(statement, tab_param, pane_param)
        {
            return Some(value);
        }

        let static_source = LuaStaticSource {
            source: body,
            max_start: start,
        };
        if let Some(value) = lua_static_string_return_from_statement(statement) {
            return Some(NativeLuaWindowTitle::Static(value));
        }
        if let Some(value) = lua_static_string_variable_return_from_statement(
            statement,
            static_source,
            outer_static_source,
        ) {
            return Some(NativeLuaWindowTitle::Static(value));
        }
        if let Some(value) = lua_dynamic_window_title_return_from_statement(
            body,
            start,
            statement,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            outer_static_source,
        ) {
            return Some(value);
        }
        if let Some(value) = lua_static_string_concat_return_from_statement(
            body,
            start,
            statement,
            outer_static_source,
        ) {
            return Some(NativeLuaWindowTitle::Static(value));
        }
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_window_title_conditional_return_from_function_body(
    body: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowTitle> {
    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;

    for start in starts {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some((if_branches, else_body, rest_after_if)) =
            lua_static_if_condition_and_body_branches_and_else_from_statement(statement)
        else {
            continue;
        };

        let mut else_parts = else_body
            .and_then(|else_body| {
                lua_static_window_title_first_return_parts_from_nested_body(
                    body,
                    else_body,
                    tab_param,
                    pane_param,
                    tabs_param,
                    panes_param,
                    outer_static_source,
                )
            })
            .or_else(|| {
                lua_static_window_title_fallback_return_parts_after_if(
                    body,
                    rest_after_if,
                    tab_param,
                    pane_param,
                    tabs_param,
                    panes_param,
                    outer_static_source,
                )
            })?;

        for (condition, if_body) in if_branches.into_iter().rev() {
            let condition = lua_window_title_condition_from_expression(
                condition,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                Some(LuaStaticSource {
                    source: body,
                    max_start: start,
                }),
            )?;
            let parts = lua_static_window_title_first_return_parts_from_nested_body(
                body,
                if_body,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                outer_static_source,
            )?;
            else_parts = vec![NativeLuaWindowTitlePart::Conditional {
                condition,
                parts,
                else_parts: Some(else_parts),
            }];
        }

        return Some(NativeLuaWindowTitle::Concat(else_parts));
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_window_title_first_return_parts_from_nested_body(
    outer_body: &str,
    nested_body: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    let nested_start = lua_source_slice_start_offset(outer_body, nested_body)?;
    for start in
        lua_top_level_statement_start_indices_before_offset(nested_body, nested_body.len())?
    {
        let statement = lua_trim_start_comments(nested_body.get(start..)?)?;
        if let Some(parts) = lua_window_title_return_parts_from_statement(
            outer_body,
            nested_start.checked_add(start)?,
            statement,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            outer_static_source,
        ) {
            return Some(parts);
        }
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_window_title_fallback_return_parts_after_if(
    outer_body: &str,
    rest_after_if: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    let rest_start = lua_source_slice_start_offset(outer_body, rest_after_if)?;
    for start in
        lua_top_level_statement_start_indices_before_offset(rest_after_if, rest_after_if.len())?
    {
        let statement = lua_trim_start_comments(rest_after_if.get(start..)?)?;
        if lua_static_if_condition_and_body_branches_from_statement(statement).is_some() {
            return None;
        }
        if let Some(parts) = lua_window_title_return_parts_from_statement(
            outer_body,
            rest_start.checked_add(start)?,
            statement,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            outer_static_source,
        ) {
            return Some(parts);
        }
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_window_title_return_parts_from_statement(
    source: &str,
    start: usize,
    statement: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    let expression = lua_static_return_expression_from_statement(statement)?;
    lua_window_title_text_parts_from_expression(
        expression,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        Some(LuaStaticSource {
            source,
            max_start: start,
        }),
        outer_static_source,
    )
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_dynamic_window_title_return_from_statement(
    source: &str,
    start: usize,
    statement: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowTitle> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_end_statement_separator(rest);
    let static_source = LuaStaticSource {
        source,
        max_start: start,
    };
    let parts = lua_window_title_text_parts_from_expression(
        rest,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        Some(static_source),
        outer_static_source,
    )?;
    Some(NativeLuaWindowTitle::Concat(parts))
}

fn lua_window_title_event_field_return_from_statement(
    statement: &str,
    tab_param: &str,
    pane_param: &str,
) -> Option<NativeLuaWindowTitle> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let tab_title = format!("{tab_param}.tab_title");
    if let Some(after_tab_title) = rest.strip_prefix(&tab_title) {
        if !lua_static_identifier_value_rest_is_statement_end(after_tab_title) {
            return None;
        }

        return Some(NativeLuaWindowTitle::ActiveTabTitle);
    }

    let tab_active_pane_title = format!("{tab_param}.active_pane.title");
    if let Some(after_tab_active_pane_title) = rest.strip_prefix(&tab_active_pane_title) {
        if !lua_static_identifier_value_rest_is_statement_end(after_tab_active_pane_title) {
            return None;
        }

        return Some(NativeLuaWindowTitle::ActivePaneTitle);
    }

    let pane_title = format!("{pane_param}.title");
    let after_pane_title = rest.strip_prefix(&pane_title)?;
    if !lua_static_identifier_value_rest_is_statement_end(after_pane_title) {
        return None;
    }

    Some(NativeLuaWindowTitle::ActivePaneTitle)
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_window_title_text_parts_from_expression(
    expression: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    lua_window_title_text_parts_from_expression_with_depth(
        expression,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        static_source,
        outer_static_source,
        0,
    )
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_window_title_text_parts_from_expression_with_depth(
    expression: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    let expression = lua_trim_start_comments(expression.trim())?;
    let expression = lua_tostring_argument_from_query(expression).unwrap_or(expression);
    if let Some(part) = lua_window_title_text_part_from_expression(
        expression,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        static_source,
        outer_static_source,
    ) {
        return Some(vec![part]);
    }

    if let Some(static_source) = static_source
        && let Some(parts) = lua_window_title_conditional_assignment_parts_before_offset(
            static_source.source,
            expression,
            static_source.max_start,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            outer_static_source,
        )
    {
        return Some(parts);
    }

    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            expression,
            static_source.max_start,
        )
    {
        return lua_window_title_text_parts_from_expression_with_depth(
            value,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            Some(static_source),
            outer_static_source,
            depth + 1,
        );
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            outer_static_source.source,
            expression,
            outer_static_source.max_start,
        )
    {
        return lua_window_title_text_parts_from_expression_with_depth(
            value,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            static_source,
            Some(outer_static_source),
            depth + 1,
        );
    }

    if let Some(parts) = lua_window_title_helper_call_parts_from_expression(
        expression,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        outer_static_source,
        depth + 1,
    ) {
        return Some(parts);
    }

    if expression.contains("..") {
        let mut parts = Vec::new();
        let mut has_dynamic_part = false;
        for segment in split_lua_string_concat_segments(expression)? {
            let segment = lua_trim_start_comments(segment.trim())?;
            let segment = lua_trim_end_statement_separator(segment);
            if let Some(part) = lua_window_title_text_part_from_expression(
                segment,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                static_source,
                outer_static_source,
            ) {
                has_dynamic_part = true;
                parts.push(part);
                continue;
            }
            if let Some(segment_parts) = lua_window_title_text_parts_from_expression_with_depth(
                segment,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                static_source,
                outer_static_source,
                depth + 1,
            ) {
                has_dynamic_part |= segment_parts
                    .iter()
                    .any(|part| !matches!(part, NativeLuaWindowTitlePart::Static(_)));
                parts.extend(segment_parts);
                continue;
            }
            let value = lua_static_string_value_from_expression(
                static_source,
                outer_static_source,
                segment,
            )?;
            parts.push(NativeLuaWindowTitlePart::Static(value));
        }

        if has_dynamic_part {
            return Some(parts);
        }
    }

    if let Some(value) =
        lua_static_string_value_from_expression(static_source, outer_static_source, expression)
    {
        return Some(vec![NativeLuaWindowTitlePart::Static(value)]);
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_window_title_text_part_from_expression(
    expression: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowTitlePart> {
    let expression = lua_trim_start_comments(expression.trim())?;
    let pane_user_vars = format!("{pane_param}.user_vars");
    if let Some(rest) = expression.strip_prefix(&pane_user_vars)
        && let Some(name) =
            lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
    {
        return Some(NativeLuaWindowTitlePart::ActivePaneUserVar { name });
    }

    let pane_progress = format!("{pane_param}.progress");
    if let Some(rest) = expression.strip_prefix(&pane_progress)
        && let Some(field) = lua_tab_title_active_pane_progress_field_from_rest(rest)
    {
        return Some(NativeLuaWindowTitlePart::ActivePaneProgress { field });
    }

    for (field, part) in lua_window_title_active_pane_text_parts() {
        let pane_field = format!("{pane_param}.{field}");
        if let Some(rest) = expression.strip_prefix(&pane_field)
            && lua_static_identifier_value_rest_is_statement_end(rest)
        {
            return Some(part);
        }
    }

    let tab_title = format!("{tab_param}.tab_title");
    if let Some(rest) = expression.strip_prefix(&tab_title)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaWindowTitlePart::ActiveTabTitle);
    }

    let tab_id = format!("{tab_param}.tab_id");
    if let Some(rest) = expression.strip_prefix(&tab_id)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaWindowTitlePart::ActiveTabId);
    }

    if let Some(tab_index_offset) =
        lua_window_title_tab_index_offset_from_expression(expression, tab_param)
    {
        return Some(NativeLuaWindowTitlePart::ActiveTabIndex {
            offset: tab_index_offset,
        });
    }

    let tab_window_title = format!("{tab_param}.window_title");
    if let Some(rest) = expression.strip_prefix(&tab_window_title)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaWindowTitlePart::WindowTitle);
    }

    let tab_active_pane_user_vars = format!("{tab_param}.active_pane.user_vars");
    if let Some(rest) = expression.strip_prefix(&tab_active_pane_user_vars)
        && let Some(name) =
            lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
    {
        return Some(NativeLuaWindowTitlePart::ActivePaneUserVar { name });
    }

    let tab_active_pane_progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = expression.strip_prefix(&tab_active_pane_progress)
        && let Some(field) = lua_tab_title_active_pane_progress_field_from_rest(rest)
    {
        return Some(NativeLuaWindowTitlePart::ActivePaneProgress { field });
    }

    for (field, part) in lua_window_title_active_pane_text_parts() {
        let tab_active_pane_field = format!("{tab_param}.active_pane.{field}");
        if let Some(rest) = expression.strip_prefix(&tab_active_pane_field)
            && lua_static_identifier_value_rest_is_statement_end(rest)
        {
            return Some(part);
        }
    }

    if let Some(receiver) = lua_identifier_literal_from_query(expression) {
        let rest = expression.get(receiver.len()..)?;
        if lua_window_title_active_pane_alias_before_offset(
            static_source,
            receiver,
            tab_param,
            pane_param,
        )
        .unwrap_or(false)
        {
            let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
            if let Some(rest) = rest.strip_prefix("user_vars")
                && let Some(name) = lua_static_pane_user_var_name_from_rest(
                    static_source,
                    outer_static_source,
                    rest,
                )
            {
                return Some(NativeLuaWindowTitlePart::ActivePaneUserVar { name });
            }

            if let Some(rest) = rest.strip_prefix("progress")
                && let Some(field) = lua_tab_title_active_pane_progress_field_from_rest(rest)
            {
                return Some(NativeLuaWindowTitlePart::ActivePaneProgress { field });
            }

            for (field, part) in lua_window_title_active_pane_text_parts() {
                if let Some(rest) = rest.strip_prefix(field)
                    && lua_static_identifier_value_rest_is_statement_end(rest)
                {
                    return Some(part);
                }
            }
        }

        if lua_window_title_active_pane_progress_alias_before_offset(
            static_source,
            receiver,
            tab_param,
            pane_param,
        ) == Some(true)
        {
            return lua_tab_title_active_pane_progress_field_from_rest(rest)
                .map(|field| NativeLuaWindowTitlePart::ActivePaneProgress { field });
        }

        if lua_window_title_active_pane_user_vars_alias_before_offset(
            static_source,
            outer_static_source,
            receiver,
            tab_param,
            pane_param,
        ) == Some(true)
            && let Some(name) =
                lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
        {
            return Some(NativeLuaWindowTitlePart::ActivePaneUserVar { name });
        }
    }

    let tab_count = format!("#{tabs_param}");
    if let Some(rest) = expression.strip_prefix(&tab_count)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaWindowTitlePart::TabCount);
    }

    let pane_count = format!("#{panes_param}");
    if let Some(rest) = expression.strip_prefix(&pane_count)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaWindowTitlePart::PaneCount);
    }

    lua_window_title_tab_index_format_part_from_expression(expression, tab_param, tabs_param)
}

fn lua_window_title_active_pane_text_parts() -> [(&'static str, NativeLuaWindowTitlePart); 6] {
    [
        ("pane_id", NativeLuaWindowTitlePart::ActivePaneId),
        (
            "domain_name",
            NativeLuaWindowTitlePart::ActivePaneDomainName,
        ),
        (
            "foreground_process_name",
            NativeLuaWindowTitlePart::ActivePaneForegroundProcessName,
        ),
        (
            "current_working_dir",
            NativeLuaWindowTitlePart::ActivePaneCurrentWorkingDir,
        ),
        ("tty_name", NativeLuaWindowTitlePart::ActivePaneTtyName),
        ("title", NativeLuaWindowTitlePart::ActivePaneTitle),
    ]
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_window_title_helper_call_parts_from_expression(
    expression: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    let outer_static_source = outer_static_source?;
    let function_name = lua_identifier_literal_from_query(expression)?;
    let rest = expression.get(function_name.len()..)?;
    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    let arguments = split_lua_top_level_arguments(arguments)?;
    let [argument] = arguments.as_slice() else {
        return None;
    };
    let argument = lua_trim_start_comments(argument.trim())?;
    let argument_name = lua_identifier_literal_from_query(argument)?;
    if argument_name != tab_param {
        return None;
    }
    if !lua_static_identifier_value_rest_is_statement_end(argument.get(argument_name.len()..)?) {
        return None;
    }

    lua_static_window_title_helper_function_parts_before_offset(
        outer_static_source.source,
        function_name,
        outer_static_source.max_start,
        outer_static_source,
        pane_param,
        tabs_param,
        panes_param,
        depth + 1,
    )
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_static_window_title_helper_function_parts_before_offset(
    source: &str,
    function_name: &str,
    max_start: usize,
    outer_static_source: LuaStaticSource<'_>,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    depth: usize,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let Some(statement) = lua_top_level_function_statement_from_index(source, start) else {
            continue;
        };
        if let Some(parts) = lua_window_title_helper_function_parts_from_statement(
            statement,
            function_name,
            outer_static_source,
            pane_param,
            tabs_param,
            panes_param,
            depth + 1,
        ) {
            selected = Some(parts);
        }
    }

    selected
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_window_title_helper_function_parts_from_statement(
    statement: &str,
    function_name: &str,
    outer_static_source: LuaStaticSource<'_>,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    depth: usize,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH || !lua_source_keyword_at(statement, 0, "function") {
        return None;
    }

    let rest = lua_trim_start_comments(statement.get("function".len()..)?)?;
    let name = lua_identifier_literal_from_query(rest)?;
    if name != function_name {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(name.len()..)?)?.strip_prefix('(')?;
    let (params, body_start) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let params = split_lua_top_level_arguments(params)?;
    let [first_param] = params.as_slice() else {
        return None;
    };
    let first_param = lua_function_param_identifier(first_param)?;
    let body = lua_static_function_body_until_end(body_start)?;
    lua_window_title_return_text_parts_from_function_body(
        body,
        first_param,
        pane_param,
        tabs_param,
        panes_param,
        Some(outer_static_source),
        depth + 1,
    )
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_window_title_return_text_parts_from_function_body(
    body: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    if let Some(parts) = lua_window_title_explicit_title_fallback_parts_from_function_body(
        body,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        outer_static_source,
        depth + 1,
    ) {
        return Some(parts);
    }

    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some(expression) = lua_static_return_expression_from_statement(statement) else {
            continue;
        };
        if let Some(parts) = lua_window_title_text_parts_from_expression_with_depth(
            expression,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            Some(LuaStaticSource {
                source: body,
                max_start: start,
            }),
            outer_static_source,
            depth + 1,
        ) {
            return Some(parts);
        }
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_window_title_explicit_title_fallback_parts_from_function_body(
    body: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    for (position, start) in starts.iter().enumerate() {
        let statement = lua_trim_start_comments(body.get(*start..)?)?;
        let Some((branches, else_body, _)) =
            lua_static_if_condition_and_body_branches_and_else_from_statement(statement)
        else {
            continue;
        };
        let Some((condition, if_body)) = branches.first().copied() else {
            continue;
        };
        let Some(condition_variable) = lua_tab_title_non_empty_condition_variable(condition) else {
            continue;
        };
        let Some(returned_variable) = lua_static_return_identifier_from_function_body(if_body)
        else {
            continue;
        };
        if returned_variable != condition_variable {
            continue;
        }

        let Some(assigned_value) =
            lua_static_expression_variable_assignment_before_offset_from_query(
                body,
                condition_variable,
                *start,
            )
        else {
            continue;
        };
        let if_static_source = LuaStaticSource {
            source: body,
            max_start: *start,
        };
        if !matches!(
            lua_window_title_text_part_from_expression(
                assigned_value,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                Some(if_static_source),
                outer_static_source,
            ),
            Some(NativeLuaWindowTitlePart::ActiveTabTitle)
        ) {
            continue;
        }

        let fallback_parts = if let Some(else_body) = else_body {
            lua_static_window_title_first_return_parts_from_nested_body(
                body,
                else_body,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                outer_static_source,
            )
        } else {
            let mut fallback_parts = None;
            for fallback_start in starts.iter().skip(position + 1) {
                let fallback_statement = lua_trim_start_comments(body.get(*fallback_start..)?)?;
                let Some(fallback_expression) =
                    lua_static_return_expression_from_statement(fallback_statement)
                else {
                    continue;
                };
                let fallback_static_source = LuaStaticSource {
                    source: body,
                    max_start: *fallback_start,
                };
                fallback_parts = lua_window_title_text_parts_from_expression_with_depth(
                    fallback_expression,
                    tab_param,
                    pane_param,
                    tabs_param,
                    panes_param,
                    Some(fallback_static_source),
                    outer_static_source,
                    depth + 1,
                );
                if fallback_parts.is_some() {
                    break;
                }
            }
            fallback_parts
        };

        let Some(fallback_parts) = fallback_parts else {
            continue;
        };
        if matches!(
            fallback_parts.as_slice(),
            [NativeLuaWindowTitlePart::ActivePaneTitle]
        ) {
            return Some(vec![
                NativeLuaWindowTitlePart::ActiveTabTitleOrActivePaneTitle,
            ]);
        }
    }

    None
}

fn lua_window_title_active_pane_alias_before_offset(
    static_source: Option<LuaStaticSource<'_>>,
    alias: &str,
    tab_param: &str,
    pane_param: &str,
) -> Option<bool> {
    let static_source = static_source?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        alias,
        static_source.max_start,
    )?;
    let value = value.trim();
    Some(value == format!("{tab_param}.active_pane") || value == pane_param)
}

fn lua_window_title_active_pane_user_vars_alias_before_offset(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    alias: &str,
    tab_param: &str,
    pane_param: &str,
) -> Option<bool> {
    let static_source = static_source?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        alias,
        static_source.max_start,
    )?;
    lua_window_title_active_pane_user_vars_expression_from_query(
        value,
        tab_param,
        pane_param,
        Some(static_source),
        outer_static_source,
    )
}

fn lua_window_title_active_pane_user_vars_expression_from_query(
    expression: &str,
    tab_param: &str,
    pane_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    _outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    let expression = lua_trim_start_comments(expression.trim())?;

    let pane_user_vars = format!("{pane_param}.user_vars");
    if let Some(rest) = expression.strip_prefix(&pane_user_vars) {
        return Some(lua_static_identifier_value_rest_is_statement_end(rest));
    }

    let tab_active_pane_user_vars = format!("{tab_param}.active_pane.user_vars");
    if let Some(rest) = expression.strip_prefix(&tab_active_pane_user_vars) {
        return Some(lua_static_identifier_value_rest_is_statement_end(rest));
    }

    let receiver = lua_identifier_literal_from_query(expression)?;
    let rest = expression.get(receiver.len()..)?;
    if lua_window_title_active_pane_alias_before_offset(
        static_source,
        receiver,
        tab_param,
        pane_param,
    ) != Some(true)
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let rest = rest.strip_prefix("user_vars")?;
    Some(lua_static_identifier_value_rest_is_statement_end(rest))
}

fn lua_window_title_active_pane_progress_alias_before_offset(
    static_source: Option<LuaStaticSource<'_>>,
    alias: &str,
    tab_param: &str,
    pane_param: &str,
) -> Option<bool> {
    let static_source = static_source?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        alias,
        static_source.max_start,
    )?;
    lua_window_title_active_pane_progress_expression_from_query(
        value,
        tab_param,
        pane_param,
        Some(static_source),
    )
}

fn lua_window_title_active_pane_progress_expression_from_query(
    expression: &str,
    tab_param: &str,
    pane_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    let expression = lua_trim_start_comments(expression.trim())?;

    let pane_progress = format!("{pane_param}.progress");
    if let Some(rest) = expression.strip_prefix(&pane_progress) {
        return Some(lua_static_identifier_value_rest_is_statement_end(rest));
    }

    let tab_active_pane_progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = expression.strip_prefix(&tab_active_pane_progress) {
        return Some(lua_static_identifier_value_rest_is_statement_end(rest));
    }

    let receiver = lua_identifier_literal_from_query(expression)?;
    let rest = expression.get(receiver.len()..)?;
    if lua_window_title_active_pane_alias_before_offset(
        static_source,
        receiver,
        tab_param,
        pane_param,
    ) != Some(true)
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let rest = rest.strip_prefix("progress")?;
    Some(lua_static_identifier_value_rest_is_statement_end(rest))
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_window_title_conditional_assignment_parts_before_offset(
    source: &str,
    expression: &str,
    max_start: usize,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    let variable = lua_identifier_literal_from_query(expression)?;
    if !lua_static_identifier_value_rest_is_statement_end(expression.get(variable.len()..)?) {
        return None;
    }

    'statements: for start in
        lua_top_level_statement_start_indices_before_offset(source, max_start)?
    {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        let Some((branches, else_body, _)) =
            lua_static_if_condition_and_body_branches_and_else_from_statement(statement)
        else {
            continue;
        };
        let mut parsed_branches = Vec::new();
        for (condition, body) in &branches {
            let Some(condition) = lua_window_title_condition_from_expression(
                condition,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                Some(LuaStaticSource {
                    source,
                    max_start: start,
                }),
            ) else {
                continue 'statements;
            };
            let Some(value) = lua_static_expression_variable_assignment_before_offset_from_query(
                body,
                variable,
                body.len(),
            ) else {
                continue 'statements;
            };
            let branch_static_source = LuaStaticSource {
                source: body,
                max_start: body.len(),
            };
            let parts = lua_window_title_assignment_parts(
                source,
                start,
                variable,
                value,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                branch_static_source,
                outer_static_source,
            )?;
            parsed_branches.push((condition, parts));
        }

        let mut else_parts = lua_window_title_else_assignment_parts(
            else_body,
            source,
            start,
            variable,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            outer_static_source,
        );
        if else_body.is_none() && else_parts.is_none() {
            else_parts = lua_window_title_previous_assignment_parts(
                source,
                start,
                variable,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                outer_static_source,
            );
        }
        for (condition, parts) in parsed_branches.into_iter().rev() {
            else_parts = Some(vec![NativeLuaWindowTitlePart::Conditional {
                condition,
                parts,
                else_parts,
            }]);
        }
        if let Some(parts) = else_parts {
            return Some(parts);
        }
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_window_title_else_assignment_parts(
    else_body: Option<&str>,
    source: &str,
    branch_start: usize,
    variable: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    let else_body = else_body?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        else_body,
        variable,
        else_body.len(),
    )?;
    let static_source = LuaStaticSource {
        source: else_body,
        max_start: else_body.len(),
    };
    lua_window_title_assignment_parts(
        source,
        branch_start,
        variable,
        value,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        static_source,
        outer_static_source,
    )
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_window_title_assignment_parts(
    source: &str,
    branch_start: usize,
    variable: &str,
    value: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    let value = lua_trim_start_comments(value.trim())?;
    let value = lua_trim_end_statement_separator(value);
    let Some(segments) = split_lua_string_concat_segments(value) else {
        if lua_window_title_expression_is_variable(value, variable) {
            return lua_window_title_previous_assignment_parts(
                source,
                branch_start,
                variable,
                tab_param,
                pane_param,
                tabs_param,
                panes_param,
                outer_static_source,
            );
        }
        return lua_window_title_text_parts_from_expression(
            value,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            Some(static_source),
            outer_static_source,
        );
    };

    if !segments
        .iter()
        .any(|segment| lua_window_title_expression_is_variable(segment, variable))
    {
        return lua_window_title_text_parts_from_expression(
            value,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            Some(static_source),
            outer_static_source,
        );
    }

    let previous_parts = lua_window_title_previous_assignment_parts(
        source,
        branch_start,
        variable,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        outer_static_source,
    )?;

    let mut parts = Vec::new();
    for segment in segments {
        let segment = lua_trim_start_comments(segment.trim())?;
        let segment = lua_trim_end_statement_separator(segment);
        if lua_window_title_expression_is_variable(segment, variable) {
            parts.extend(previous_parts.clone());
            continue;
        }

        let segment_parts = lua_window_title_text_parts_from_expression(
            segment,
            tab_param,
            pane_param,
            tabs_param,
            panes_param,
            Some(static_source),
            outer_static_source,
        )?;
        parts.extend(segment_parts);
    }

    (!parts.is_empty()).then_some(parts)
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_window_title_previous_assignment_parts(
    source: &str,
    before_start: usize,
    variable: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaWindowTitlePart>> {
    let previous_value = lua_static_expression_variable_assignment_before_offset_from_query(
        source,
        variable,
        before_start,
    )?;
    let previous_static_source = LuaStaticSource {
        source,
        max_start: before_start,
    };
    lua_window_title_text_parts_from_expression(
        previous_value,
        tab_param,
        pane_param,
        tabs_param,
        panes_param,
        Some(previous_static_source),
        outer_static_source,
    )
}

fn lua_window_title_expression_is_variable(expression: &str, variable: &str) -> bool {
    let Some(expression) = lua_trim_start_comments(expression.trim()) else {
        return false;
    };
    let Some(rest) = expression.strip_prefix(variable) else {
        return false;
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return false;
    }
    lua_static_identifier_value_rest_is_statement_end(rest)
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_window_title_condition_from_expression(
    condition: &str,
    tab_param: &str,
    pane_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowTitleCondition> {
    let condition = lua_trim_start_comments(condition.trim())?;
    for zoomed in [
        format!("{tab_param}.active_pane.is_zoomed"),
        format!("{pane_param}.is_zoomed"),
    ] {
        if let Some(rest) = condition.strip_prefix(&zoomed)
            && lua_static_identifier_value_rest_is_statement_end(rest)
        {
            return Some(NativeLuaWindowTitleCondition::ActivePaneIsZoomed);
        }
    }

    if let Some(receiver) = lua_identifier_literal_from_query(condition) {
        let rest = condition.get(receiver.len()..)?;
        if lua_window_title_active_pane_alias_before_offset(
            static_source,
            receiver,
            tab_param,
            pane_param,
        )
        .unwrap_or(false)
        {
            let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
            let rest = rest.strip_prefix("is_zoomed")?;
            if lua_static_identifier_value_rest_is_statement_end(rest) {
                return Some(NativeLuaWindowTitleCondition::ActivePaneIsZoomed);
            }
        }
    }

    if let Some((field, present)) = lua_window_title_active_pane_progress_field_presence_condition(
        condition,
        tab_param,
        pane_param,
        static_source,
    ) {
        return Some(
            NativeLuaWindowTitleCondition::ActivePaneProgressFieldPresence { field, present },
        );
    }

    if lua_window_title_active_pane_progress_indeterminate_condition(
        condition,
        tab_param,
        pane_param,
        static_source,
    ) {
        return Some(NativeLuaWindowTitleCondition::ActivePaneProgressIndeterminate);
    }

    if let Some((name, present)) = lua_window_title_active_pane_user_var_presence_condition(
        condition,
        tab_param,
        pane_param,
        static_source,
    ) {
        return Some(NativeLuaWindowTitleCondition::ActivePaneUserVarPresence { name, present });
    }

    if let Some(count) = lua_window_title_count_greater_than_condition(condition, tabs_param) {
        return Some(NativeLuaWindowTitleCondition::TabCountGreaterThan(count));
    }
    if let Some(count) = lua_window_title_count_greater_than_condition(condition, panes_param) {
        return Some(NativeLuaWindowTitleCondition::PaneCountGreaterThan(count));
    }

    None
}

fn lua_window_title_active_pane_progress_indeterminate_condition(
    condition: &str,
    tab_param: &str,
    pane_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> bool {
    let pane_progress = format!("{pane_param}.progress");
    if let Some(rest) = condition.strip_prefix(&pane_progress) {
        return lua_tab_title_progress_indeterminate_rest_is_complete(rest);
    }

    let tab_active_pane_progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = condition.strip_prefix(&tab_active_pane_progress) {
        return lua_tab_title_progress_indeterminate_rest_is_complete(rest);
    }

    let Some(receiver) = lua_identifier_literal_from_query(condition) else {
        return false;
    };
    let Some(rest) = condition.get(receiver.len()..) else {
        return false;
    };
    if lua_window_title_active_pane_alias_before_offset(
        static_source,
        receiver,
        tab_param,
        pane_param,
    ) == Some(true)
    {
        let Some(rest) = lua_trim_start_comments(rest).and_then(|rest| rest.strip_prefix('.'))
        else {
            return false;
        };
        let Some(rest) = rest.strip_prefix("progress") else {
            return false;
        };
        return lua_tab_title_progress_indeterminate_rest_is_complete(rest);
    }

    if lua_window_title_active_pane_progress_alias_before_offset(
        static_source,
        receiver,
        tab_param,
        pane_param,
    ) == Some(true)
    {
        return lua_tab_title_progress_indeterminate_rest_is_complete(rest);
    }

    false
}

fn lua_window_title_active_pane_user_var_presence_condition(
    condition: &str,
    tab_param: &str,
    pane_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<(String, bool)> {
    let pane_user_vars = format!("{pane_param}.user_vars");
    if let Some(rest) = condition.strip_prefix(&pane_user_vars) {
        return lua_tab_title_user_var_presence_rest(static_source, None, rest);
    }

    let tab_active_pane_user_vars = format!("{tab_param}.active_pane.user_vars");
    if let Some(rest) = condition.strip_prefix(&tab_active_pane_user_vars) {
        return lua_tab_title_user_var_presence_rest(static_source, None, rest);
    }

    let receiver = lua_identifier_literal_from_query(condition)?;
    let rest = condition.get(receiver.len()..)?;
    if lua_window_title_active_pane_alias_before_offset(
        static_source,
        receiver,
        tab_param,
        pane_param,
    ) == Some(true)
    {
        let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
        let rest = rest.strip_prefix("user_vars")?;
        return lua_tab_title_user_var_presence_rest(static_source, None, rest);
    }

    if lua_window_title_active_pane_user_vars_alias_before_offset(
        static_source,
        None,
        receiver,
        tab_param,
        pane_param,
    ) == Some(true)
    {
        return lua_tab_title_user_var_presence_rest(static_source, None, rest);
    }

    None
}

fn lua_window_title_active_pane_progress_field_presence_condition(
    condition: &str,
    tab_param: &str,
    pane_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<(NativeLuaTabTitleProgressField, bool)> {
    let pane_progress = format!("{pane_param}.progress");
    if let Some(rest) = condition.strip_prefix(&pane_progress) {
        return lua_tab_title_progress_field_presence_rest(rest);
    }

    let tab_active_pane_progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = condition.strip_prefix(&tab_active_pane_progress) {
        return lua_tab_title_progress_field_presence_rest(rest);
    }

    let receiver = lua_identifier_literal_from_query(condition)?;
    let rest = condition.get(receiver.len()..)?;
    if lua_window_title_active_pane_alias_before_offset(
        static_source,
        receiver,
        tab_param,
        pane_param,
    ) == Some(true)
    {
        let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
        let rest = rest.strip_prefix("progress")?;
        return lua_tab_title_progress_field_presence_rest(rest);
    }

    if lua_window_title_active_pane_progress_alias_before_offset(
        static_source,
        receiver,
        tab_param,
        pane_param,
    ) == Some(true)
    {
        return lua_tab_title_progress_field_presence_rest(rest);
    }

    None
}

fn lua_window_title_count_greater_than_condition(
    condition: &str,
    count_param: &str,
) -> Option<usize> {
    let count_expression = condition.strip_prefix('#')?;
    let rest = count_expression.strip_prefix(count_param)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('>')?)?;
    let count = lua_unsigned_integer_literal_from_query(rest)?;
    let after_count = lua_trim_start_comments(rest.get(count.len()..)?)?;
    lua_static_identifier_value_rest_is_statement_end(after_count).then(|| count.parse().ok())?
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_window_title_tab_index_format_part_from_expression(
    expression: &str,
    tab_param: &str,
    tabs_param: &str,
) -> Option<NativeLuaWindowTitlePart> {
    let (format, tab_index_offset) =
        lua_tab_index_count_format_from_expression(expression, tab_param, tabs_param)?;
    Some(NativeLuaWindowTitlePart::TabIndexAndCount {
        format,
        tab_index_offset,
    })
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_tab_index_count_format_from_expression(
    expression: &str,
    tab_param: &str,
    tabs_param: &str,
) -> Option<(NativeLuaWindowTitleNumberPairFormat, usize)> {
    let rest = expression.strip_prefix("string")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("format") || !lua_config_assignment_field_has_boundaries(rest, 0, "format")
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get("format".len()..)?)?.strip_prefix('(')?;
    let (argument_list, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let arguments = split_lua_top_level_arguments(argument_list)?;
    let [
        format_expression,
        tab_index_expression,
        tab_count_expression,
    ] = arguments.as_slice()
    else {
        return None;
    };
    let (format, _) = lua_inline_string_literal_value_and_len(format_expression.trim())?;
    let format = NativeLuaWindowTitleNumberPairFormat::parse(&format)?;
    let tab_index_offset =
        lua_window_title_tab_index_offset_from_expression(tab_index_expression.trim(), tab_param)?;
    lua_window_title_tab_count_expression(tab_count_expression.trim(), tabs_param)?;
    Some((format, tab_index_offset))
}

fn lua_window_title_tab_index_offset_from_expression(
    expression: &str,
    tab_param: &str,
) -> Option<usize> {
    let tab_index = format!("{tab_param}.tab_index");
    let rest = expression.strip_prefix(&tab_index)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    if lua_static_identifier_value_rest_is_statement_end(rest) {
        return Some(0);
    }
    let rest = lua_trim_start_comments(rest.strip_prefix('+')?)?;
    let offset = lua_unsigned_integer_literal_from_query(rest)?;
    let after_offset = lua_trim_start_comments(rest.get(offset.len()..)?)?;
    lua_static_identifier_value_rest_is_statement_end(after_offset).then(|| offset.parse().ok())?
}

fn lua_window_title_tab_count_expression(expression: &str, tabs_param: &str) -> Option<()> {
    let rest = expression.strip_prefix('#')?;
    let rest = rest.strip_prefix(tabs_param)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    lua_static_identifier_value_rest_is_statement_end(rest).then_some(())
}

fn lua_static_string_return_from_statement(statement: &str) -> Option<String> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let (value, value_len) = lua_inline_string_literal_value_and_len(rest)?;
    let rest = lua_trim_start_comments(rest.get(value_len..)?)?;
    let rest = rest.strip_prefix(';').unwrap_or(rest);
    lua_trim_start_comments(rest)?.is_empty().then_some(value)
}

fn lua_static_string_variable_return_from_statement(
    statement: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<String> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    if let Some(value) = lua_static_string_assignment_value_before_offset_from_query(
        static_source.source,
        rest,
        static_source.max_start,
    ) {
        return lua_inline_string_literal_value_and_len(value).map(|(value, _)| value);
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_string_assignment_value_before_offset_from_query(
            outer_static_source.source,
            rest,
            outer_static_source.max_start,
        )
    {
        return lua_inline_string_literal_value_and_len(value).map(|(value, _)| value);
    }

    None
}

fn lua_static_string_concat_return_from_statement(
    source: &str,
    start: usize,
    statement: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<String> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let rest = rest.strip_suffix(';').unwrap_or(rest).trim();
    if !rest.contains("..") {
        return None;
    }

    let static_source = LuaStaticSource {
        source,
        max_start: start,
    };
    let mut value = String::new();
    for segment in split_lua_string_concat_segments(rest)? {
        let segment = lua_trim_start_comments(segment.trim())?;
        let segment = lua_trim_end_statement_separator(segment);
        value.push_str(&parse_maybe_static_query_text_with_static_sources(
            Some(static_source),
            outer_static_source,
            segment,
        )?);
    }
    (!value.is_empty()).then_some(value)
}

fn split_lua_string_concat_segments(value: &str) -> Option<Vec<&str>> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;

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

        if value[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&value[index + 2..])
            {
                let content_and_rest = &value[index + 2 + content_start..];
                block_comment_end = Some(
                    content_and_rest
                        .find(&closing)
                        .map_or(value.len(), |close_index| {
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
            '.' if value[index..].starts_with("..") => {
                let segment = value.get(start..index)?.trim();
                if segment.is_empty() {
                    return None;
                }
                segments.push(segment);
                start = index + "..".len();
            }
            _ => {}
        }

        if character == '['
            && let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&value[index..])
        {
            let content_and_rest = &value[index + content_start..];
            long_bracket_end = Some(
                content_and_rest
                    .find(&closing)
                    .map_or(value.len(), |close_index| {
                        index + content_start + close_index + closing.len()
                    }),
            );
        }
    }

    let segment = value.get(start..)?.trim();
    if segment.is_empty() {
        return None;
    }
    segments.push(segment);

    (segments.len() > 1).then_some(segments)
}

fn lua_trim_end_statement_separator(value: &str) -> &str {
    let value = value.trim_end();
    if let Some(value) = value.strip_suffix(';') {
        return value.trim_end();
    }
    value
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_tab_title_return_from_function_body(
    body: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    hover_param: Option<&str>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    if let Some(value) = lua_static_tab_title_conditional_return_from_function_body(
        body,
        tab_param,
        tabs_param,
        panes_param,
        hover_param,
        outer_static_source,
    ) {
        return Some(value);
    }

    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(value) = lua_static_tab_title_return_from_statement_as_lua_title(
            body,
            start,
            statement,
            tab_param,
            tabs_param,
            panes_param,
            outer_static_source,
        ) {
            return Some(value);
        }
    }

    None
}

fn lua_static_new_tab_button_click_return_from_function_body(
    body: &str,
    window_param: Option<&str>,
    pane_param: Option<&str>,
    button_param: Option<&str>,
    default_action_param: Option<&str>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaNewTabButtonClick> {
    let allow_default = lua_static_new_tab_button_click_allow_default_from_function_body(
        body,
        button_param,
        outer_static_source,
    )?;
    let perform_default_action = match (window_param, pane_param, default_action_param) {
        (Some(window_param), Some(pane_param), Some(default_action_param)) => {
            lua_static_new_tab_button_click_performs_default_action(
                body,
                window_param,
                pane_param,
                default_action_param,
            )?
        }
        _ => false,
    };
    Some(NativeLuaNewTabButtonClick {
        allow_default,
        perform_default_action,
    })
}

fn lua_static_new_tab_button_click_allow_default_from_function_body(
    body: &str,
    button_param: Option<&str>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaNewTabButtonClickAllowDefault> {
    if let Some(allow_default) =
        lua_static_bool_return_from_function_body(body, outer_static_source)
    {
        return Some(NativeLuaNewTabButtonClickAllowDefault::Static(
            allow_default,
        ));
    }

    let button_param = button_param?;
    if let Some(defaults) = lua_static_new_tab_button_click_direct_button_return_from_function_body(
        body,
        button_param,
        outer_static_source,
    ) {
        return Some(NativeLuaNewTabButtonClickAllowDefault::ButtonConditions { defaults });
    }

    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some((branches, else_body, rest_after_if)) =
            lua_static_if_condition_and_body_branches_and_else_from_statement(statement)
        else {
            continue;
        };
        if !lua_trim_end_statement_separator(rest_after_if)
            .trim()
            .is_empty()
        {
            continue;
        }
        let mut button_branches = Vec::new();
        let static_source = LuaStaticSource {
            source: body,
            max_start: start,
        };
        for (condition, branch_body) in branches {
            let (button, comparison) = lua_new_tab_button_click_button_condition(
                condition,
                button_param,
                static_source,
                outer_static_source,
            )?;
            let allow_default =
                lua_static_bool_return_from_function_body(branch_body, outer_static_source)?;
            button_branches.push(LuaNewTabButtonClickButtonBranch {
                button,
                comparison,
                allow_default,
            });
        }
        let else_allow_default = else_body
            .and_then(|else_body| {
                lua_static_bool_return_from_function_body(else_body, outer_static_source)
            })
            .unwrap_or(true);
        return Some(NativeLuaNewTabButtonClickAllowDefault::ButtonConditions {
            defaults: NativeLuaNewTabButtonClickButtonDefaults::from_lua_branches(
                &button_branches,
                else_allow_default,
            ),
        });
    }

    None
}

fn lua_static_new_tab_button_click_direct_button_return_from_function_body(
    body: &str,
    button_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaNewTabButtonClickButtonDefaults> {
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some(expression) = lua_static_return_expression_from_statement(statement) else {
            continue;
        };
        let static_source = LuaStaticSource {
            source: body,
            max_start: start,
        };
        let (button, comparison) = lua_new_tab_button_click_button_condition(
            expression,
            button_param,
            static_source,
            outer_static_source,
        )?;
        return Some(NativeLuaNewTabButtonClickButtonDefaults::from_lua_branches(
            &[LuaNewTabButtonClickButtonBranch {
                button,
                comparison,
                allow_default: true,
            }],
            false,
        ));
    }

    None
}

fn lua_static_new_tab_button_click_performs_default_action(
    body: &str,
    window_param: &str,
    pane_param: &str,
    default_action_param: &str,
) -> Option<bool> {
    lua_static_new_tab_button_click_performs_default_action_with_static_source(
        body,
        window_param,
        pane_param,
        default_action_param,
        None,
    )
}

fn lua_static_new_tab_button_click_performs_default_action_with_static_source(
    body: &str,
    window_param: &str,
    pane_param: &str,
    default_action_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    for (index, start) in starts.iter().copied().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(body.len());
        let statement = lua_trim_start_comments(body.get(start..end)?)?;
        let static_source = LuaStaticSource {
            source: body,
            max_start: start,
        };
        if lua_new_tab_button_click_statement_performs_default_action(
            statement,
            window_param,
            pane_param,
            default_action_param,
            static_source,
            outer_static_source,
        )? {
            return Some(true);
        }
    }
    Some(false)
}

fn lua_new_tab_button_click_statement_performs_default_action(
    statement: &str,
    window_param: &str,
    pane_param: &str,
    default_action_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    if let Some(action) = lua_callback_statement_perform_action_query_with_static_sources(
        statement,
        window_param,
        pane_param,
        static_source,
        outer_static_source,
    ) {
        return Some(lua_new_tab_button_click_expression_is_default_action_param(
            action,
            default_action_param,
            static_source,
            outer_static_source,
        ));
    }
    if let Some((branches, rest)) =
        lua_static_if_condition_and_body_branches_from_statement(statement)
    {
        if !lua_trim_end_statement_separator(rest).trim().is_empty() {
            return Some(false);
        }
        for (condition, body) in branches {
            if lua_new_tab_button_click_condition_is_default_action(
                condition,
                default_action_param,
                static_source,
                outer_static_source,
            )? && lua_static_new_tab_button_click_performs_default_action_with_static_source(
                body,
                window_param,
                pane_param,
                default_action_param,
                Some(static_source),
            )? {
                return Some(true);
            }
        }
    }
    Some(false)
}

fn lua_new_tab_button_click_condition_is_default_action(
    condition: &str,
    default_action_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    let condition = lua_trim_start_comments(condition.trim())?;
    if let Some(present) = lua_new_tab_button_click_default_action_nil_presence_condition(
        condition,
        default_action_param,
        static_source,
        outer_static_source,
    ) {
        return Some(present);
    }
    Some(lua_new_tab_button_click_expression_is_default_action_param(
        condition,
        default_action_param,
        static_source,
        outer_static_source,
    ))
}

fn lua_new_tab_button_click_default_action_nil_presence_condition(
    condition: &str,
    default_action_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    if let Some((left, right)) = condition.split_once("~=") {
        return lua_new_tab_button_click_default_action_nil_presence(
            left,
            right,
            default_action_param,
            static_source,
            outer_static_source,
            true,
        );
    }
    if let Some((left, right)) = condition.split_once("==") {
        return lua_new_tab_button_click_default_action_nil_presence(
            left,
            right,
            default_action_param,
            static_source,
            outer_static_source,
            false,
        );
    }
    None
}

fn lua_new_tab_button_click_default_action_nil_presence(
    left: &str,
    right: &str,
    default_action_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    present: bool,
) -> Option<bool> {
    let left_is_default_action = lua_new_tab_button_click_expression_is_default_action_param(
        left,
        default_action_param,
        static_source,
        outer_static_source,
    );
    let right_is_default_action = lua_new_tab_button_click_expression_is_default_action_param(
        right,
        default_action_param,
        static_source,
        outer_static_source,
    );
    let left_is_nil = lua_new_tab_button_click_expression_is_nil(left);
    let right_is_nil = lua_new_tab_button_click_expression_is_nil(right);

    ((left_is_default_action && right_is_nil) || (left_is_nil && right_is_default_action))
        .then_some(present)
}

fn lua_new_tab_button_click_expression_is_nil(expression: &str) -> bool {
    let Some(expression) = lua_trim_start_comments(expression.trim()) else {
        return false;
    };
    let Some(rest) = expression.strip_prefix("nil") else {
        return false;
    };
    lua_static_identifier_value_rest_is_statement_end(rest)
}

fn lua_new_tab_button_click_expression_is_default_action_param(
    expression: &str,
    default_action_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> bool {
    lua_new_tab_button_click_expression_is_default_action_param_with_depth(
        expression,
        default_action_param,
        static_source,
        outer_static_source,
        0,
    )
}

fn lua_new_tab_button_click_expression_is_default_action_param_with_depth(
    expression: &str,
    default_action_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> bool {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return false;
    }
    if lua_static_identifier_expression_matches(expression, default_action_param) {
        return true;
    }
    if let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        expression,
        static_source.max_start,
    ) {
        return lua_new_tab_button_click_expression_is_default_action_param_with_depth(
            value,
            default_action_param,
            static_source,
            outer_static_source,
            depth + 1,
        );
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            outer_static_source.source,
            expression,
            outer_static_source.max_start,
        )
    {
        return lua_new_tab_button_click_expression_is_default_action_param_with_depth(
            value,
            default_action_param,
            outer_static_source,
            None,
            depth + 1,
        );
    }
    false
}

fn lua_new_tab_button_click_button_condition(
    condition: &str,
    button_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<(MouseButton, LuaNewTabButtonClickButtonComparison)> {
    let condition = lua_trim_start_comments(condition.trim())?;
    let (left, right, comparison) = if let Some((left, right)) = condition.split_once("~=") {
        (left, right, LuaNewTabButtonClickButtonComparison::NotEquals)
    } else {
        let (left, right) = condition.split_once("==")?;
        (left, right, LuaNewTabButtonClickButtonComparison::Equals)
    };
    let left = lua_trim_start_comments(left.trim())?;
    let right = lua_trim_start_comments(right.trim())?;

    if lua_new_tab_button_click_expression_is_button_param(left, button_param, static_source) {
        return Some((
            lua_new_tab_button_click_button_from_expression(
                right,
                Some(static_source),
                outer_static_source,
            )?,
            comparison,
        ));
    }
    if lua_new_tab_button_click_expression_is_button_param(right, button_param, static_source) {
        return Some((
            lua_new_tab_button_click_button_from_expression(
                left,
                Some(static_source),
                outer_static_source,
            )?,
            comparison,
        ));
    }
    None
}

fn lua_new_tab_button_click_expression_is_button_param(
    expression: &str,
    button_param: &str,
    static_source: LuaStaticSource<'_>,
) -> bool {
    lua_new_tab_button_click_expression_is_button_param_with_depth(
        expression,
        button_param,
        static_source,
        0,
    )
}

fn lua_new_tab_button_click_expression_is_button_param_with_depth(
    expression: &str,
    button_param: &str,
    static_source: LuaStaticSource<'_>,
    depth: usize,
) -> bool {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return false;
    }
    if lua_static_identifier_expression_matches(expression, button_param) {
        return true;
    }
    let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        expression,
        static_source.max_start,
    ) else {
        return false;
    };
    lua_new_tab_button_click_expression_is_button_param_with_depth(
        value,
        button_param,
        static_source,
        depth + 1,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LuaNewTabButtonClickButtonComparison {
    Equals,
    NotEquals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LuaNewTabButtonClickButtonBranch {
    button: MouseButton,
    comparison: LuaNewTabButtonClickButtonComparison,
    allow_default: bool,
}

impl LuaNewTabButtonClickButtonBranch {
    fn matches(self, button: MouseButton) -> bool {
        match self.comparison {
            LuaNewTabButtonClickButtonComparison::Equals => button == self.button,
            LuaNewTabButtonClickButtonComparison::NotEquals => button != self.button,
        }
    }
}

fn lua_new_tab_button_click_button_from_expression(
    expression: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<MouseButton> {
    match lua_static_string_value_from_expression(static_source, outer_static_source, expression)?
        .as_str()
    {
        "Left" | "left" => Some(MouseButton::Left),
        "Right" | "right" => Some(MouseButton::Right),
        "Middle" | "middle" => Some(MouseButton::Middle),
        _ => None,
    }
}

fn lua_static_open_uri_return_from_function_body(
    body: &str,
    window_param: &str,
    pane_param: &str,
    uri_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaOpenUri> {
    if let Some(allow_default) =
        lua_static_bool_return_from_function_body(body, outer_static_source)
    {
        return Some(NativeLuaOpenUri::Static { allow_default });
    }

    lua_static_open_uri_prefix_return_from_function_body(
        body,
        window_param,
        pane_param,
        uri_param,
        outer_static_source,
    )
}

fn lua_static_bool_return_from_function_body(
    body: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some(expression) = lua_static_return_expression_from_statement(statement) else {
            continue;
        };
        let allow_default = parse_maybe_static_query_bool_with_static_sources(
            Some(LuaStaticSource {
                source: body,
                max_start: start,
            }),
            outer_static_source,
            expression,
        )?;
        return Some(allow_default);
    }

    None
}

fn lua_static_command_palette_entries_return_from_function_body(
    body: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeCommandPaletteEntry>> {
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some(expression) = lua_static_return_expression_from_statement(statement) else {
            continue;
        };
        return native_command_palette_entries_lua_table_from_expression(
            Some(LuaStaticSource {
                source: body,
                max_start: start,
            }),
            outer_static_source,
            expression,
        );
    }

    None
}

fn native_command_palette_entries_lua_table_from_expression(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<NativeCommandPaletteEntry>> {
    native_command_palette_entries_lua_table_from_expression_with_depth(
        static_source,
        outer_static_source,
        value,
        0,
    )
}

fn native_command_palette_entries_lua_table_from_expression_with_depth(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    depth: usize,
) -> Option<Vec<NativeCommandPaletteEntry>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }
    let value = value.trim();
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return native_command_palette_entries_lua_table_from_expression_with_depth(
            Some(static_source),
            outer_static_source,
            value,
            depth + 1,
        );
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            outer_static_source.source,
            value,
            outer_static_source.max_start,
        )
    {
        return native_command_palette_entries_lua_table_from_expression_with_depth(
            static_source,
            Some(outer_static_source),
            value,
            depth + 1,
        );
    }

    native_command_palette_entries_lua_table_from_query(static_source, outer_static_source, value)
}

fn native_command_palette_entries_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<NativeCommandPaletteEntry>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut entries = Vec::new();
    let mut indexed_entries = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !entries.is_empty() || index == 0 || indexed_entries.contains_key(&index) {
                return None;
            }
            indexed_entries.insert(
                index,
                native_command_palette_entry_lua_table_from_query(
                    static_source,
                    outer_static_source,
                    value.trim(),
                )?,
            );
            continue;
        }

        if !indexed_entries.is_empty() {
            return None;
        }
        entries.push(native_command_palette_entry_lua_table_from_query(
            static_source,
            outer_static_source,
            field,
        )?);
    }

    if !indexed_entries.is_empty() {
        return (1..=indexed_entries.len())
            .map(|index| indexed_entries.remove(&index))
            .collect();
    }

    Some(entries)
}

fn native_command_palette_entry_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeCommandPaletteEntry> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut brief = None;
    let mut doc = None;
    let mut icon = None;
    let mut action = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_sources(
            static_source,
            outer_static_source,
            name.trim(),
        )?;
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "brief" => {
                if brief.is_some() {
                    return None;
                }
                brief = Some(parse_maybe_static_query_text_with_static_sources(
                    static_source,
                    outer_static_source,
                    value,
                )?);
            }
            "doc" => {
                if doc.is_some() {
                    return None;
                }
                doc = Some(parse_maybe_static_query_text_with_static_sources(
                    static_source,
                    outer_static_source,
                    value,
                )?);
            }
            "icon" => {
                if icon.is_some() {
                    return None;
                }
                icon = Some(parse_maybe_static_query_text_with_static_sources(
                    static_source,
                    outer_static_source,
                    value,
                )?);
            }
            "action" => {
                if action.is_some() {
                    return None;
                }
                action = Some(native_command_palette_entry_action_from_query(
                    static_source,
                    outer_static_source,
                    value,
                )?);
            }
            _ => return None,
        }
    }

    Some(NativeCommandPaletteEntry {
        brief: brief?,
        doc,
        icon,
        key_assignment: None,
        action: action?,
    })
}

fn native_command_palette_entry_action_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCommand> {
    native_command_palette_entry_action_from_query_with_depth(
        static_source,
        outer_static_source,
        value,
        0,
    )
}

fn native_command_palette_entry_action_from_query_with_depth(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    depth: usize,
) -> Option<WindowCommand> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }
    let value = value.trim();
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_action_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return native_command_palette_entry_action_from_query_with_depth(
            Some(static_source),
            outer_static_source,
            value,
            depth + 1,
        );
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_action_assignment_value_before_offset_from_query(
            outer_static_source.source,
            value,
            outer_static_source.max_start,
        )
    {
        return native_command_palette_entry_action_from_query_with_depth(
            static_source,
            Some(outer_static_source),
            value,
            depth + 1,
        );
    }

    native_key_assignment_command_from_query(static_source, value)
        .or_else(|| native_key_assignment_command_from_query(outer_static_source, value))
        .or_else(|| native_key_assignment_command_from_query(None, value))
}

fn lua_static_open_uri_prefix_return_from_function_body(
    body: &str,
    window_param: &str,
    pane_param: &str,
    uri_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaOpenUri> {
    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    for start in starts {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some((if_branches, _)) =
            lua_static_if_condition_and_body_branches_from_statement(statement)
        else {
            continue;
        };
        let (condition, if_body) = if_branches.first().copied()?;
        let prefix =
            lua_open_uri_prefix_condition_from_expression(body, start, condition, uri_param)?;
        let allow_default = lua_static_bool_return_from_function_body(
            if_body,
            Some(LuaStaticSource {
                source: body,
                max_start: start,
            })
            .or(outer_static_source),
        )?;
        let action = lua_static_open_uri_prefix_action_from_function_body(
            if_body,
            window_param,
            pane_param,
            uri_param,
            outer_static_source,
        )?;
        return Some(NativeLuaOpenUri::UriPrefix {
            prefix,
            allow_default,
            action,
        });
    }

    None
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_static_open_uri_prefix_action_from_function_body(
    body: &str,
    window_param: &str,
    pane_param: &str,
    uri_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Option<NativeLuaOpenUriAction>> {
    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    for (index, start) in starts.iter().copied().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(body.len());
        let statement = lua_trim_start_comments(body.get(start..end)?)?;
        let Some(action_query) =
            lua_callback_statement_perform_action_query(statement, window_param, pane_param)
        else {
            continue;
        };
        let action = lua_static_open_uri_action_from_query(
            Some(LuaStaticSource {
                source: body,
                max_start: start,
            }),
            outer_static_source,
            action_query,
            uri_param,
        )?;
        return Some(Some(action));
    }

    Some(None)
}

fn lua_static_open_uri_action_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    uri_param: &str,
) -> Option<NativeLuaOpenUriAction> {
    let indexed_action;
    let value = if let Some(value) = strip_wezterm_action_prefix(value) {
        value
    } else if let Some(value) = strip_wezterm_action_index_prefix(value) {
        indexed_action = value;
        indexed_action.as_str()
    } else {
        value
    };
    let value = value.trim();
    let action_name = lua_identifier_literal_from_query(value)?;
    if normalized_action_name_query(action_name) != "spawncommandinnewwindow" {
        return None;
    }
    let rest = lua_trim_start_comments(value.get(action_name.len()..)?)?;
    let table = if rest.starts_with('{') {
        rest.strip_prefix('{')?.strip_suffix('}')?.trim()
    } else if rest.starts_with('(') {
        let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
        let (arguments, after) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_end_statement_separator(after).trim().is_empty() {
            return None;
        }
        let arguments = split_lua_top_level_arguments(arguments)?;
        let [table] = arguments.as_slice() else {
            return None;
        };
        table.trim().strip_prefix('{')?.strip_suffix('}')?.trim()
    } else {
        return None;
    };

    let mut args = None;
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
        if !key.eq_ignore_ascii_case("args") || args.is_some() {
            return None;
        }
        args = Some(lua_static_open_uri_spawn_args_from_query(
            static_source,
            outer_static_source,
            value.trim(),
            uri_param,
        )?);
    }

    let args = args?;
    (!args.is_empty()).then_some(NativeLuaOpenUriAction::SpawnCommandInNewWindow { args })
}

fn lua_static_open_uri_spawn_args_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    uri_param: &str,
) -> Option<Vec<NativeLuaOpenUriArg>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut args = Vec::new();
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let value = if let Some((key, value)) = split_lua_table_assignment_from_field(field) {
            split_lua_table_array_index_from_query(key.trim())?;
            value.trim()
        } else {
            field
        };
        if let Some(value) =
            lua_static_string_value_from_expression(static_source, outer_static_source, value)
        {
            args.push(NativeLuaOpenUriArg::Static(value));
            continue;
        }
        if lua_static_open_uri_suffix_value_from_expression(static_source, value, uri_param)? {
            args.push(NativeLuaOpenUriArg::UriSuffix);
            continue;
        }
        return None;
    }
    Some(args)
}

fn lua_static_open_uri_suffix_value_from_expression(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    uri_param: &str,
) -> Option<bool> {
    if lua_open_uri_suffix_from_expression(value, uri_param)? {
        return Some(true);
    }
    let Some(static_source) = static_source else {
        return Some(false);
    };
    let name = lua_identifier_literal_from_query(value.trim())?;
    if !lua_static_identifier_value_rest_is_statement_end(value.trim().get(name.len()..)?) {
        return Some(false);
    }
    lua_open_uri_suffix_assignment_before_offset_from_query(
        static_source.source,
        name,
        static_source.max_start,
        uri_param,
    )
    .map(|matched| matched.is_some())
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_open_uri_suffix_assignment_before_offset_from_query(
    source: &str,
    variable: &str,
    max_start: usize,
    uri_param: &str,
) -> Option<Option<()>> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if lua_open_uri_suffix_assignment_from_statement(statement, variable, uri_param)? {
            selected = Some(());
        }
    }
    Some(selected)
}

fn lua_open_uri_suffix_assignment_from_statement(
    statement: &str,
    variable: &str,
    uri_param: &str,
) -> Option<bool> {
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let assigned_variable = lua_identifier_literal_from_query(rest)?;
    if assigned_variable != variable {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest.get(assigned_variable.len()..)?)?;
    let value = lua_trim_start_comments(rest)?.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(value)?;
    lua_open_uri_suffix_from_expression(value, uri_param)
}

fn lua_open_uri_suffix_from_expression(value: &str, uri_param: &str) -> Option<bool> {
    let value = lua_trim_start_comments(value.trim())?;
    let Some(rest) = value.strip_prefix(uri_param) else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("sub") || !lua_config_assignment_field_has_boundaries(rest, 0, "sub") {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest.get("sub".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, after) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_end_statement_separator(after).trim().is_empty() {
        return Some(false);
    }
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [start] = arguments.as_slice() else {
        return Some(false);
    };
    lua_open_uri_suffix_start_expression_from_query(start.trim())
}

fn lua_open_uri_suffix_start_expression_from_query(value: &str) -> Option<bool> {
    let value = lua_trim_start_comments(value.trim())?;
    let name = lua_identifier_literal_from_query(value)?;
    let rest = lua_trim_start_comments(value.get(name.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('+')?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('1')?)?;
    Some(lua_static_identifier_value_rest_is_statement_end(rest))
}

fn lua_open_uri_prefix_condition_from_expression(
    body: &str,
    if_start: usize,
    condition: &str,
    uri_param: &str,
) -> Option<String> {
    let condition = condition.trim();
    let variable = lua_identifier_literal_from_query(condition)?;
    let rest = lua_trim_start_comments(condition.get(variable.len()..)?)?;
    let rest = rest.strip_prefix("==")?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = rest.strip_prefix('1')?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    lua_open_uri_find_prefix_assignment_before_offset_from_query(
        body, variable, if_start, uri_param,
    )
}

fn lua_open_uri_find_prefix_assignment_before_offset_from_query(
    body: &str,
    variable: &str,
    max_start: usize,
    uri_param: &str,
) -> Option<String> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(body, max_start)? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(prefix) =
            lua_open_uri_find_prefix_assignment_from_statement(statement, variable, uri_param)
        {
            selected = Some(prefix);
        }
    }
    selected
}

fn lua_open_uri_find_prefix_assignment_from_statement(
    statement: &str,
    variable: &str,
    uri_param: &str,
) -> Option<String> {
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let assigned_variable = lua_identifier_literal_from_query(rest)?;
    if assigned_variable != variable {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(assigned_variable.len()..)?)?;
    let value = if let Some(rest) = rest.strip_prefix(',') {
        let equals = rest.find('=')?;
        rest.get(equals + 1..)?
    } else {
        lua_trim_start_comments(rest)?.strip_prefix('=')?
    };
    let value = lua_top_level_statement_value_from_query(value)?;
    lua_open_uri_find_prefix_from_expression(value, uri_param)
}

fn lua_open_uri_find_prefix_from_expression(value: &str, uri_param: &str) -> Option<String> {
    let value = lua_trim_start_comments(value.trim())?;
    let rest = value.strip_prefix(uri_param)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("find") || !lua_config_assignment_field_has_boundaries(rest, 0, "find") {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get("find".len()..)?)?;
    let prefix_expression = if rest.starts_with('(') {
        let (arguments, after) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(after)?.is_empty() {
            return None;
        }
        split_lua_top_level_arguments(arguments)?
            .into_iter()
            .next()?
    } else {
        let (_, literal_len) = lua_inline_string_literal_value_and_len(rest)?;
        let expression = rest.get(..literal_len)?;
        if !lua_trim_start_comments(rest.get(literal_len..)?)?.is_empty() {
            return None;
        }
        expression
    };

    lua_static_string_value_from_expression(None, None, prefix_expression)
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_tab_title_return_from_statement_as_lua_title(
    source: &str,
    start: usize,
    statement: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    let static_source = LuaStaticSource {
        source,
        max_start: start,
    };
    if let Some(value) = lua_tab_title_event_field_return_from_statement(
        statement,
        tab_param,
        tabs_param,
        panes_param,
    ) {
        return Some(value);
    }
    if let Some(value) = lua_dynamic_tab_title_concat_return_from_statement(
        source,
        start,
        statement,
        tab_param,
        tabs_param,
        panes_param,
        outer_static_source,
    ) {
        return Some(value);
    }
    if let Some(value) = lua_dynamic_tab_title_format_return_from_statement(
        source,
        start,
        statement,
        tab_param,
        tabs_param,
        panes_param,
        outer_static_source,
    ) {
        return Some(value);
    }
    if let Some(value) = lua_dynamic_tab_title_text_return_from_statement(
        source,
        start,
        statement,
        tab_param,
        tabs_param,
        panes_param,
        outer_static_source,
    ) {
        return Some(value);
    }
    if let Some(value) =
        lua_static_tab_title_return_from_statement(statement, static_source, outer_static_source)
    {
        return Some(NativeLuaTabTitle::Static(value));
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_static_tab_title_conditional_return_from_function_body(
    body: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    hover_param: Option<&str>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    let mut branches = Vec::new();
    let mut fallback = None;

    for start in starts {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some((if_branches, else_body, rest_after_if)) =
            lua_static_if_condition_and_body_branches_and_else_from_statement(statement)
        {
            let fallback_return =
                lua_static_tab_title_fallback_return_statement_after_if(body, rest_after_if);
            for (condition, if_body) in if_branches {
                let condition = lua_tab_title_condition_from_expression(
                    condition,
                    tab_param,
                    tabs_param,
                    panes_param,
                    hover_param,
                    Some(LuaStaticSource {
                        source: body,
                        max_start: start,
                    }),
                    outer_static_source,
                )?;
                let title = lua_static_tab_title_first_return_from_nested_body(
                    body,
                    if_body,
                    tab_param,
                    tabs_param,
                    panes_param,
                    outer_static_source,
                )
                .or_else(|| {
                    let (_, shared_prefix, return_statement) = fallback_return?;
                    lua_static_tab_title_return_with_branch_assignments(
                        body,
                        start,
                        if_body,
                        shared_prefix,
                        return_statement,
                        tab_param,
                        tabs_param,
                        panes_param,
                        outer_static_source,
                    )
                })?;
                branches.push(NativeLuaTabTitleConditionalBranch { condition, title });
            }
            if fallback.is_none()
                && let Some(title) = else_body
                    .and_then(|else_body| {
                        lua_static_tab_title_first_return_from_nested_body(
                            body,
                            else_body,
                            tab_param,
                            tabs_param,
                            panes_param,
                            outer_static_source,
                        )
                        .or_else(|| {
                            let (_, shared_prefix, return_statement) = fallback_return?;
                            lua_static_tab_title_return_with_branch_assignments(
                                body,
                                start,
                                else_body,
                                shared_prefix,
                                return_statement,
                                tab_param,
                                tabs_param,
                                panes_param,
                                outer_static_source,
                            )
                        })
                    })
                    .or_else(|| {
                        lua_static_tab_title_fallback_return_after_if(
                            body,
                            rest_after_if,
                            tab_param,
                            tabs_param,
                            panes_param,
                            outer_static_source,
                        )
                    })
            {
                fallback = Some(Box::new(title));
                break;
            }
            continue;
        }

        if branches.is_empty() {
            continue;
        }

        if let Some(title) = lua_static_tab_title_return_from_statement_as_lua_title(
            body,
            start,
            statement,
            tab_param,
            tabs_param,
            panes_param,
            outer_static_source,
        ) {
            fallback = Some(Box::new(title));
            break;
        }
    }

    Some(NativeLuaTabTitle::Conditional {
        branches,
        fallback: fallback?,
    })
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_tab_title_fallback_return_after_if(
    outer_body: &str,
    rest_after_if: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    let (start, _, statement) =
        lua_static_tab_title_fallback_return_statement_after_if(outer_body, rest_after_if)?;
    lua_static_tab_title_return_from_statement_as_lua_title(
        outer_body,
        start,
        statement,
        tab_param,
        tabs_param,
        panes_param,
        outer_static_source,
    )
}

fn lua_static_tab_title_fallback_return_statement_after_if<'a>(
    outer_body: &'a str,
    rest_after_if: &'a str,
) -> Option<(usize, &'a str, &'a str)> {
    let starts =
        lua_top_level_statement_start_indices_before_offset(rest_after_if, rest_after_if.len())?;
    let rest_start = lua_source_slice_start_offset(outer_body, rest_after_if)?;
    for start in starts {
        let statement = lua_trim_start_comments(rest_after_if.get(start..)?)?;
        if lua_static_if_condition_and_body_branches_from_statement(statement).is_some() {
            return None;
        }
        if lua_static_return_expression_from_statement(statement).is_none() {
            continue;
        }
        let shared_prefix = rest_after_if.get(..start)?;
        return Some((rest_start.checked_add(start)?, shared_prefix, statement));
    }
    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_static_tab_title_return_with_branch_assignments(
    body: &str,
    if_start: usize,
    if_body: &str,
    shared_prefix: &str,
    return_statement: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    let mut branch_source = String::new();
    branch_source.push_str(body.get(..if_start)?);
    branch_source.push('\n');
    branch_source.push_str(if_body);
    branch_source.push('\n');
    branch_source.push_str(shared_prefix);
    branch_source.push('\n');
    let return_start = branch_source.len();
    branch_source.push_str(return_statement);
    lua_static_tab_title_return_from_statement_as_lua_title(
        &branch_source,
        return_start,
        return_statement,
        tab_param,
        tabs_param,
        panes_param,
        outer_static_source,
    )
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_static_tab_title_first_return_from_nested_body(
    outer_body: &str,
    nested_body: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    let nested_body_start = lua_source_slice_start_offset(outer_body, nested_body)?;
    for start in
        lua_top_level_statement_start_indices_before_offset(nested_body, nested_body.len())?
    {
        let statement = lua_trim_start_comments(nested_body.get(start..)?)?;
        let outer_start = nested_body_start.checked_add(start)?;
        if let Some(title) = lua_static_tab_title_return_from_statement_as_lua_title(
            outer_body,
            outer_start,
            statement,
            tab_param,
            tabs_param,
            panes_param,
            outer_static_source,
        ) {
            return Some(title);
        }
    }

    None
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_tab_title_condition_from_expression(
    condition: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    hover_param: Option<&str>,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitleCondition> {
    let condition = lua_trim_start_comments(condition.trim())?;
    if let Some(hover_param) = hover_param
        && let Some(rest) = condition.strip_prefix(hover_param)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaTabTitleCondition::IsHover);
    }

    if lua_tab_title_active_pane_bool_field_condition(
        condition,
        tab_param,
        "is_zoomed",
        static_source,
        outer_static_source,
    ) {
        return Some(NativeLuaTabTitleCondition::ActivePaneIsZoomed);
    }

    if lua_tab_title_active_pane_progress_indeterminate_condition(
        condition,
        tab_param,
        static_source,
        outer_static_source,
    ) {
        return Some(NativeLuaTabTitleCondition::ActivePaneProgressIndeterminate);
    }

    if let Some((field, present)) = lua_tab_title_active_pane_progress_field_presence_condition(
        condition,
        tab_param,
        static_source,
        outer_static_source,
    ) {
        return Some(
            NativeLuaTabTitleCondition::ActivePaneProgressFieldPresence { field, present },
        );
    }

    if let Some((name, present)) = lua_tab_title_active_pane_user_var_presence_condition(
        condition,
        tab_param,
        static_source,
        outer_static_source,
    ) {
        return Some(NativeLuaTabTitleCondition::ActivePaneUserVarPresence { name, present });
    }

    for (field, parsed) in [
        ("is_active", NativeLuaTabTitleCondition::IsActive),
        ("is_last_active", NativeLuaTabTitleCondition::IsLastActive),
    ] {
        let path = format!("{tab_param}.{field}");
        if let Some(rest) = condition.strip_prefix(&path)
            && lua_static_identifier_value_rest_is_statement_end(rest)
        {
            return Some(parsed);
        }
    }

    if let Some(count) = lua_window_title_count_greater_than_condition(condition, tabs_param) {
        return Some(NativeLuaTabTitleCondition::TabCountGreaterThan(count));
    }
    if let Some(count) = lua_window_title_count_greater_than_condition(condition, panes_param) {
        return Some(NativeLuaTabTitleCondition::PaneCountGreaterThan(count));
    }

    None
}

fn lua_tab_title_active_pane_bool_field_condition(
    condition: &str,
    tab_param: &str,
    field: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> bool {
    let path = format!("{tab_param}.active_pane.{field}");
    if let Some(rest) = condition.strip_prefix(&path)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return true;
    }

    let Some(receiver) = lua_identifier_literal_from_query(condition) else {
        return false;
    };
    let Some(rest) = condition.get(receiver.len()..) else {
        return false;
    };
    if lua_tab_title_active_pane_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) != Some(true)
    {
        return false;
    }
    let Some(rest) = lua_trim_start_comments(rest).and_then(|rest| rest.strip_prefix('.')) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(field) else {
        return false;
    };
    lua_static_identifier_value_rest_is_statement_end(rest)
}

fn lua_tab_title_active_pane_progress_indeterminate_condition(
    condition: &str,
    tab_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> bool {
    let progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = condition.strip_prefix(&progress) {
        return lua_tab_title_progress_indeterminate_rest_is_complete(rest);
    }

    let Some(receiver) = lua_identifier_literal_from_query(condition) else {
        return false;
    };
    let Some(rest) = condition.get(receiver.len()..) else {
        return false;
    };
    if lua_tab_title_active_pane_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) == Some(true)
    {
        let Some(rest) = lua_trim_start_comments(rest).and_then(|rest| rest.strip_prefix('.'))
        else {
            return false;
        };
        let Some(rest) = rest.strip_prefix("progress") else {
            return false;
        };
        return lua_tab_title_progress_indeterminate_rest_is_complete(rest);
    }

    if lua_tab_title_active_pane_progress_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) == Some(true)
    {
        return lua_tab_title_progress_indeterminate_rest_is_complete(rest);
    }

    false
}

fn lua_tab_title_progress_indeterminate_rest_is_complete(rest: &str) -> bool {
    let Some(rest) = lua_trim_start_comments(rest).and_then(|rest| rest.strip_prefix("==")) else {
        return false;
    };
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some(literal) = lua_quoted_string_literal_from_query(rest)
        .or_else(|| lua_long_bracket_literal_from_query(rest))
    else {
        return false;
    };
    let Some(value) = parse_maybe_quoted_query_text(literal) else {
        return false;
    };
    value == "Indeterminate"
        && lua_trim_start_comments(rest.get(literal.len()..).unwrap_or_default())
            .is_some_and(str::is_empty)
}

fn lua_tab_title_active_pane_progress_field_presence_condition(
    condition: &str,
    tab_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<(NativeLuaTabTitleProgressField, bool)> {
    let progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = condition.strip_prefix(&progress) {
        return lua_tab_title_progress_field_presence_rest(rest);
    }

    let receiver = lua_identifier_literal_from_query(condition)?;
    let rest = condition.get(receiver.len()..)?;
    if lua_tab_title_active_pane_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) == Some(true)
    {
        let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
        let rest = rest.strip_prefix("progress")?;
        return lua_tab_title_progress_field_presence_rest(rest);
    }

    if lua_tab_title_active_pane_progress_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) == Some(true)
    {
        return lua_tab_title_progress_field_presence_rest(rest);
    }

    None
}

fn lua_tab_title_progress_field_presence_rest(
    rest: &str,
) -> Option<(NativeLuaTabTitleProgressField, bool)> {
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    let parsed = match field {
        "Percentage" => NativeLuaTabTitleProgressField::Percentage,
        "Error" => NativeLuaTabTitleProgressField::Error,
        _ => return None,
    };
    let rest = lua_trim_start_comments(rest.get(field.len()..)?)?;
    let (rest, present) = if let Some(rest) = rest.strip_prefix("~=") {
        (rest, true)
    } else if let Some(rest) = rest.strip_prefix("==") {
        (rest, false)
    } else {
        return None;
    };
    let rest = lua_trim_start_comments(rest)?;
    let rest = rest.strip_prefix("nil")?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    Some((parsed, present))
}

fn lua_tab_title_active_pane_user_var_presence_condition(
    condition: &str,
    tab_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<(String, bool)> {
    let user_vars = format!("{tab_param}.active_pane.user_vars");
    if let Some(rest) = condition.strip_prefix(&user_vars) {
        return lua_tab_title_user_var_presence_rest(static_source, outer_static_source, rest);
    }

    let receiver = lua_identifier_literal_from_query(condition)?;
    let rest = condition.get(receiver.len()..)?;
    if lua_tab_title_active_pane_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) == Some(true)
    {
        let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
        let rest = rest.strip_prefix("user_vars")?;
        return lua_tab_title_user_var_presence_rest(static_source, outer_static_source, rest);
    }

    if lua_tab_title_active_pane_user_vars_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) == Some(true)
    {
        return lua_tab_title_user_var_presence_rest(static_source, outer_static_source, rest);
    }

    None
}

fn lua_tab_title_user_var_presence_rest(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    rest: &str,
) -> Option<(String, bool)> {
    let (name, rest) =
        lua_tab_title_user_var_name_and_rest_from_rest(static_source, outer_static_source, rest)?;
    let rest = lua_trim_start_comments(rest)?;
    let (rest, present) = if let Some(rest) = rest.strip_prefix("~=") {
        (rest, true)
    } else if let Some(rest) = rest.strip_prefix("==") {
        (rest, false)
    } else {
        return None;
    };
    let rest = lua_trim_start_comments(rest)?.strip_prefix("nil")?;
    lua_static_identifier_value_rest_is_statement_end(rest).then_some((name, present))
}

fn lua_tab_title_user_var_name_and_rest_from_rest<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    rest: &'a str,
) -> Option<(String, &'a str)> {
    let rest = lua_trim_start_comments(rest)?;
    if let Some(rest) = rest.strip_prefix('.') {
        let name = lua_identifier_literal_from_query(rest)?;
        return Some((name.to_owned(), rest.get(name.len()..)?));
    }

    let rest = rest.strip_prefix('[')?;
    let rest = lua_trim_start_comments(rest)?;
    let (name, rest) =
        lua_static_pane_user_var_bracket_key_from_query(static_source, outer_static_source, rest)?;
    let rest = lua_trim_start_comments(rest)?.strip_prefix(']')?;
    Some((name, rest))
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_tab_title_event_field_return_from_statement(
    statement: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
) -> Option<NativeLuaTabTitle> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let tab_id = format!("{tab_param}.tab_id");
    if let Some(after_tab_id) = rest.strip_prefix(&tab_id) {
        if !lua_static_identifier_value_rest_is_statement_end(after_tab_id) {
            return None;
        }
        return Some(NativeLuaTabTitle::TabId);
    }

    if let Some(tab_index_offset) =
        lua_window_title_tab_index_offset_from_expression(rest, tab_param)
    {
        return Some(NativeLuaTabTitle::TabIndex {
            offset: tab_index_offset,
        });
    }

    let tab_count = format!("#{tabs_param}");
    if let Some(after_tab_count) = rest.strip_prefix(&tab_count) {
        if !lua_static_identifier_value_rest_is_statement_end(after_tab_count) {
            return None;
        }
        return Some(NativeLuaTabTitle::TabCount);
    }

    let pane_count = format!("#{panes_param}");
    if let Some(after_pane_count) = rest.strip_prefix(&pane_count) {
        if !lua_static_identifier_value_rest_is_statement_end(after_pane_count) {
            return None;
        }
        return Some(NativeLuaTabTitle::PaneCount);
    }

    let tab_title = format!("{tab_param}.tab_title");
    if let Some(after_tab_title) = rest.strip_prefix(&tab_title) {
        if !lua_static_identifier_value_rest_is_statement_end(after_tab_title) {
            return None;
        }
        return Some(NativeLuaTabTitle::ActiveTabTitle);
    }

    let window_title = format!("{tab_param}.window_title");
    if let Some(after_window_title) = rest.strip_prefix(&window_title) {
        if !lua_static_identifier_value_rest_is_statement_end(after_window_title) {
            return None;
        }
        return Some(NativeLuaTabTitle::WindowTitle);
    }

    let active_pane_domain_name = format!("{tab_param}.active_pane.domain_name");
    if let Some(after_active_pane_domain_name) = rest.strip_prefix(&active_pane_domain_name) {
        if !lua_static_identifier_value_rest_is_statement_end(after_active_pane_domain_name) {
            return None;
        }
        return Some(NativeLuaTabTitle::ActivePaneDomainName);
    }

    let active_pane_foreground_process_name =
        format!("{tab_param}.active_pane.foreground_process_name");
    if let Some(after_active_pane_foreground_process_name) =
        rest.strip_prefix(&active_pane_foreground_process_name)
    {
        if !lua_static_identifier_value_rest_is_statement_end(
            after_active_pane_foreground_process_name,
        ) {
            return None;
        }
        return Some(NativeLuaTabTitle::ActivePaneForegroundProcessName);
    }

    let active_pane_current_working_dir = format!("{tab_param}.active_pane.current_working_dir");
    if let Some(after_active_pane_current_working_dir) =
        rest.strip_prefix(&active_pane_current_working_dir)
    {
        if !lua_static_identifier_value_rest_is_statement_end(after_active_pane_current_working_dir)
        {
            return None;
        }
        return Some(NativeLuaTabTitle::ActivePaneCurrentWorkingDir);
    }

    let active_pane_tty_name = format!("{tab_param}.active_pane.tty_name");
    if let Some(after_active_pane_tty_name) = rest.strip_prefix(&active_pane_tty_name) {
        if !lua_static_identifier_value_rest_is_statement_end(after_active_pane_tty_name) {
            return None;
        }
        return Some(NativeLuaTabTitle::ActivePaneTtyName);
    }

    let active_pane_title = format!("{tab_param}.active_pane.title");
    let after_active_pane_title = rest.strip_prefix(&active_pane_title)?;
    if !lua_static_identifier_value_rest_is_statement_end(after_active_pane_title) {
        return None;
    }

    Some(NativeLuaTabTitle::ActivePaneTitle)
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_dynamic_tab_title_concat_return_from_statement(
    source: &str,
    start: usize,
    statement: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let rest = rest.strip_suffix(';').unwrap_or(rest).trim();
    if !rest.contains("..") {
        return None;
    }

    let static_source = LuaStaticSource {
        source,
        max_start: start,
    };
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;
    for segment in split_lua_string_concat_segments(rest)? {
        let segment = lua_trim_start_comments(segment.trim())?;
        let segment = lua_trim_end_statement_separator(segment);
        if let Some(part) = lua_tab_title_text_part_from_expression(
            segment,
            tab_param,
            tabs_param,
            panes_param,
            Some(static_source),
            outer_static_source,
        ) {
            has_dynamic_part = true;
            parts.push(part);
            continue;
        }
        let value = lua_static_string_value_from_expression(
            Some(static_source),
            outer_static_source,
            segment,
        )?;
        parts.push(NativeLuaTabTitleTextPart::Static(value));
    }

    has_dynamic_part.then_some(NativeLuaTabTitle::Concat(parts))
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_tab_title_text_part_from_expression(
    expression: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitleTextPart> {
    let expression = lua_trim_start_comments(expression.trim())?;
    let tab_id = format!("{tab_param}.tab_id");
    if let Some(rest) = expression.strip_prefix(&tab_id)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaTabTitleTextPart::TabId);
    }

    if let Some(tab_index_offset) =
        lua_window_title_tab_index_offset_from_expression(expression, tab_param)
    {
        return Some(NativeLuaTabTitleTextPart::TabIndex {
            offset: tab_index_offset,
        });
    }

    let tab_count = format!("#{tabs_param}");
    if let Some(rest) = expression.strip_prefix(&tab_count)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaTabTitleTextPart::TabCount);
    }

    let pane_count = format!("#{panes_param}");
    if let Some(rest) = expression.strip_prefix(&pane_count)
        && lua_static_identifier_value_rest_is_statement_end(rest)
    {
        return Some(NativeLuaTabTitleTextPart::PaneCount);
    }

    if let Some((format, tab_index_offset)) =
        lua_tab_index_count_format_from_expression(expression, tab_param, tabs_param)
    {
        return Some(NativeLuaTabTitleTextPart::TabIndexAndCount {
            format,
            tab_index_offset,
        });
    }

    for (field, part) in [
        ("tab_title", NativeLuaTabTitleTextPart::ActiveTabTitle),
        ("window_title", NativeLuaTabTitleTextPart::WindowTitle),
    ] {
        let path = format!("{tab_param}.{field}");
        if let Some(rest) = expression.strip_prefix(&path)
            && lua_static_identifier_value_rest_is_statement_end(rest)
        {
            return Some(part);
        }
    }

    let active_pane_user_vars = format!("{tab_param}.active_pane.user_vars");
    if let Some(rest) = expression.strip_prefix(&active_pane_user_vars)
        && let Some(name) =
            lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
    {
        return Some(NativeLuaTabTitleTextPart::ActivePaneUserVar { name });
    }

    let active_pane_progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = expression.strip_prefix(&active_pane_progress)
        && let Some(field) = lua_tab_title_active_pane_progress_field_from_rest(rest)
    {
        return Some(NativeLuaTabTitleTextPart::ActivePaneProgress { field });
    }

    for (field, part) in lua_tab_title_active_pane_text_parts() {
        let path = format!("{tab_param}.active_pane.{field}");
        if let Some(rest) = expression.strip_prefix(&path)
            && lua_static_identifier_value_rest_is_statement_end(rest)
        {
            return Some(part);
        }
    }

    let receiver = lua_identifier_literal_from_query(expression)?;
    let rest = expression.get(receiver.len()..)?;
    if lua_tab_title_active_pane_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    )? {
        let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
        if let Some(rest) = rest.strip_prefix("user_vars")
            && let Some(name) =
                lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
        {
            return Some(NativeLuaTabTitleTextPart::ActivePaneUserVar { name });
        }

        if let Some(rest) = rest.strip_prefix("progress")
            && let Some(field) = lua_tab_title_active_pane_progress_field_from_rest(rest)
        {
            return Some(NativeLuaTabTitleTextPart::ActivePaneProgress { field });
        }

        for (field, part) in lua_tab_title_active_pane_text_parts() {
            if let Some(rest) = rest.strip_prefix(field)
                && lua_static_identifier_value_rest_is_statement_end(rest)
            {
                return Some(part);
            }
        }
    }

    if lua_tab_title_active_pane_user_vars_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) == Some(true)
        && let Some(name) =
            lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
    {
        return Some(NativeLuaTabTitleTextPart::ActivePaneUserVar { name });
    }

    if lua_tab_title_active_pane_progress_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    )? {
        return lua_tab_title_active_pane_progress_field_from_rest(rest)
            .map(|field| NativeLuaTabTitleTextPart::ActivePaneProgress { field });
    }

    None
}

fn lua_tab_title_active_pane_progress_field_from_rest(
    rest: &str,
) -> Option<NativeLuaTabTitleProgressField> {
    let rest = lua_trim_start_comments(rest).and_then(|rest| rest.strip_prefix('.'))?;
    let field = lua_identifier_literal_from_query(rest)?;
    if !lua_trim_start_comments(rest.get(field.len()..).unwrap_or_default())
        .is_some_and(str::is_empty)
    {
        return None;
    }

    match field {
        "Percentage" => Some(NativeLuaTabTitleProgressField::Percentage),
        "Error" => Some(NativeLuaTabTitleProgressField::Error),
        _ => None,
    }
}

fn lua_tab_title_active_pane_text_parts() -> [(&'static str, NativeLuaTabTitleTextPart); 6] {
    [
        ("pane_id", NativeLuaTabTitleTextPart::ActivePaneId),
        (
            "domain_name",
            NativeLuaTabTitleTextPart::ActivePaneDomainName,
        ),
        (
            "foreground_process_name",
            NativeLuaTabTitleTextPart::ActivePaneForegroundProcessName,
        ),
        (
            "current_working_dir",
            NativeLuaTabTitleTextPart::ActivePaneCurrentWorkingDir,
        ),
        ("tty_name", NativeLuaTabTitleTextPart::ActivePaneTtyName),
        ("title", NativeLuaTabTitleTextPart::ActivePaneTitle),
    ]
}

fn lua_tab_title_active_pane_alias_before_offset(
    static_source: Option<LuaStaticSource<'_>>,
    _outer_static_source: Option<LuaStaticSource<'_>>,
    alias: &str,
    tab_param: &str,
) -> Option<bool> {
    let static_source = static_source?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        alias,
        static_source.max_start,
    )?;
    Some(value.trim() == format!("{tab_param}.active_pane"))
}

fn lua_tab_title_active_pane_user_vars_alias_before_offset(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    alias: &str,
    tab_param: &str,
) -> Option<bool> {
    let static_source = static_source?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        alias,
        static_source.max_start,
    )?;
    lua_tab_title_active_pane_user_vars_expression_from_query(
        value,
        tab_param,
        Some(static_source),
        outer_static_source,
    )
}

fn lua_tab_title_active_pane_user_vars_expression_from_query(
    expression: &str,
    tab_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    let expression = lua_trim_start_comments(expression.trim())?;
    let active_pane_user_vars = format!("{tab_param}.active_pane.user_vars");
    if let Some(rest) = expression.strip_prefix(&active_pane_user_vars) {
        return Some(lua_static_identifier_value_rest_is_statement_end(rest));
    }

    let receiver = lua_identifier_literal_from_query(expression)?;
    let rest = expression.get(receiver.len()..)?;
    if lua_tab_title_active_pane_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) != Some(true)
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let rest = rest.strip_prefix("user_vars")?;
    Some(lua_static_identifier_value_rest_is_statement_end(rest))
}

fn lua_tab_title_active_pane_progress_alias_before_offset(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    alias: &str,
    tab_param: &str,
) -> Option<bool> {
    let static_source = static_source?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        alias,
        static_source.max_start,
    )?;
    lua_tab_title_active_pane_progress_expression_from_query(
        value,
        tab_param,
        Some(static_source),
        outer_static_source,
    )
}

fn lua_tab_title_active_pane_progress_expression_from_query(
    expression: &str,
    tab_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<bool> {
    let expression = lua_trim_start_comments(expression.trim())?;
    let active_pane_progress = format!("{tab_param}.active_pane.progress");
    if let Some(rest) = expression.strip_prefix(&active_pane_progress) {
        return Some(lua_static_identifier_value_rest_is_statement_end(rest));
    }

    let receiver = lua_identifier_literal_from_query(expression)?;
    let rest = expression.get(receiver.len()..)?;
    if lua_tab_title_active_pane_alias_before_offset(
        static_source,
        outer_static_source,
        receiver,
        tab_param,
    ) != Some(true)
    {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let rest = rest.strip_prefix("progress")?;
    Some(lua_static_identifier_value_rest_is_statement_end(rest))
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_dynamic_tab_title_format_return_from_statement(
    source: &str,
    start: usize,
    statement: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaTabTitle> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let rest = rest.strip_suffix(';').unwrap_or(rest).trim();
    let static_source = Some(LuaStaticSource {
        source,
        max_start: start,
    });
    let (items, has_dynamic_item) = native_lua_format_items_from_lua_format_items_table_query(
        static_source,
        outer_static_source,
        rest,
        tab_param,
        tabs_param,
        panes_param,
    )?;
    has_dynamic_item.then_some(NativeLuaTabTitle::Format(items))
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn native_lua_format_items_from_lua_format_items_table_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
) -> Option<(Vec<NativeLuaFormatItem>, bool)> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut items = Vec::new();
    let mut has_dynamic_item = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some(item) = native_lua_format_item_from_lua_table_query(
            static_source,
            outer_static_source,
            field,
            tab_param,
            tabs_param,
            panes_param,
        ) {
            has_dynamic_item = true;
            items.push(item);
            continue;
        }
        if let Some(text) = parse_maybe_static_query_text_with_static_sources(
            static_source,
            outer_static_source,
            field,
        ) && text == "ResetAttributes"
        {
            items.push(NativeLuaFormatItem::Static(
                NativeFormatItem::ResetAttributes,
            ));
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
            items.push(NativeLuaFormatItem::Static(item));
        } else {
            return None;
        }
    }

    Some((items, has_dynamic_item))
}
