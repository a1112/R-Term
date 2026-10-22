fn input_selector_action_from_lua_action_callback_query(
    value: &str,
) -> Option<WindowInputSelectorAction> {
    let callback = strip_lua_function_call_from_query(value, "wezterm.action_callback")
        .or_else(|| strip_lua_function_call_from_query(value, "action_callback"))?;
    let (body, window_param, pane_param, id_param, label_param) =
        lua_anonymous_function_body_and_first_four_params_from_query(callback)?;
    input_selector_action_from_lua_action_callback_body(
        body,
        window_param,
        pane_param,
        id_param,
        label_param,
    )
}
fn input_selector_action_from_lua_action_callback_body(
    body: &str,
    window_param: &str,
    pane_param: &str,
    id_param: &str,
    label_param: &str,
) -> Option<WindowInputSelectorAction> {
    let mut found = None;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some(action) = input_selector_action_from_lua_action_callback_statement(
            statement,
            window_param,
            pane_param,
            id_param,
            label_param,
        ) else {
            continue;
        };
        input_selector_merge_static_action(&mut found, action)?;
    }
    found
}

fn input_selector_action_from_lua_action_callback_statement(
    statement: &str,
    window_param: &str,
    pane_param: &str,
    id_param: &str,
    label_param: &str,
) -> Option<WindowInputSelectorAction> {
    if let Some(action) = input_selector_callback_statement_sends_pane_input_param(
        statement,
        pane_param,
        id_param,
        label_param,
    ) {
        return Some(action);
    }
    if let Some(action) = input_selector_callback_statement_switches_to_workspace(
        statement,
        window_param,
        pane_param,
        id_param,
        label_param,
    ) {
        return Some(action);
    }
    if let Some((branches, else_body, rest)) =
        lua_static_if_condition_and_body_branches_and_else_from_statement(statement)
    {
        if !lua_trim_end_statement_separator(rest).trim().is_empty() {
            return None;
        }
        let mut found = None;
        for (_, body) in branches {
            if let Some(action) = input_selector_action_from_lua_action_callback_body(
                body,
                window_param,
                pane_param,
                id_param,
                label_param,
            ) {
                input_selector_merge_static_action(&mut found, action)?;
            }
        }
        if let Some(body) = else_body
            && let Some(action) = input_selector_action_from_lua_action_callback_body(
                body,
                window_param,
                pane_param,
                id_param,
                label_param,
            )
        {
            input_selector_merge_static_action(&mut found, action)?;
        }
        return found;
    }
    None
}

fn input_selector_merge_static_action(
    found: &mut Option<WindowInputSelectorAction>,
    action: WindowInputSelectorAction,
) -> Option<()> {
    if let Some(existing) = found {
        if existing != &action {
            return None;
        }
    } else {
        *found = Some(action);
    }
    Some(())
}

fn input_selector_callback_statement_sends_pane_input_param(
    statement: &str,
    pane_param: &str,
    id_param: &str,
    label_param: &str,
) -> Option<WindowInputSelectorAction> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = statement.strip_prefix(pane_param)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let (command_name, id_action, label_action) = if rest.starts_with("send_text")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_text")
    {
        (
            "send_text",
            WindowInputSelectorAction::SendIdText,
            WindowInputSelectorAction::SendLabelText,
        )
    } else if rest.starts_with("send_paste")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_paste")
    {
        (
            "send_paste",
            WindowInputSelectorAction::SendIdPaste,
            WindowInputSelectorAction::SendLabelPaste,
        )
    } else {
        return None;
    };
    let rest = lua_trim_start_comments(rest.get(command_name.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [argument] = arguments.as_slice() else {
        return None;
    };
    let argument = lua_trim_start_comments(argument.trim())?;
    let name = lua_identifier_literal_from_query(argument)?;
    if !lua_static_identifier_value_rest_is_statement_end(argument.get(name.len()..)?) {
        return None;
    }
    if !lua_trim_end_statement_separator(rest).trim().is_empty() {
        return None;
    }
    if name == id_param {
        Some(id_action)
    } else if name == label_param {
        Some(label_action)
    } else {
        None
    }
}

fn input_selector_callback_statement_switches_to_workspace(
    statement: &str,
    window_param: &str,
    pane_param: &str,
    id_param: &str,
    label_param: &str,
) -> Option<WindowInputSelectorAction> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = statement.strip_prefix(window_param)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("perform_action")
        || !lua_config_assignment_field_has_boundaries(rest, 0, "perform_action")
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get("perform_action".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [action, pane] = arguments.as_slice() else {
        return None;
    };
    let pane = lua_trim_start_comments(pane.trim())?;
    let name = lua_identifier_literal_from_query(pane)?;
    if name != pane_param
        || !lua_static_identifier_value_rest_is_statement_end(pane.get(name.len()..)?)
        || !lua_trim_end_statement_separator(rest).trim().is_empty()
    {
        return None;
    }
    input_selector_switch_to_workspace_action_from_query(action.trim(), id_param, label_param)
}

fn input_selector_switch_to_workspace_action_from_query(
    action: &str,
    id_param: &str,
    label_param: &str,
) -> Option<WindowInputSelectorAction> {
    let indexed_action;
    let action = if let Some(action) = strip_wezterm_action_prefix(action) {
        action
    } else if let Some(action) = strip_wezterm_action_index_prefix(action) {
        indexed_action = action;
        indexed_action.as_str()
    } else {
        action
    };
    let action = action.trim();
    let action_name = lua_identifier_literal_from_query(action)?;
    if normalized_action_name_query(action_name) != "switchtoworkspace" {
        return None;
    }
    let rest = lua_trim_start_comments(action.get(action_name.len()..)?)?;
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

    let mut workspace_name = None;
    let mut cwd = None;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        if key.eq_ignore_ascii_case("name") {
            if workspace_name.is_some() {
                return None;
            }
            workspace_name = Some(input_selector_callback_value_param_from_query(
                value,
                id_param,
                label_param,
            )?);
        } else if key.eq_ignore_ascii_case("spawn") {
            if cwd.is_some() {
                return None;
            }
            cwd = input_selector_switch_to_workspace_spawn_cwd_from_query(
                value,
                id_param,
                label_param,
            )?;
        } else {
            return None;
        }
    }

    Some(WindowInputSelectorAction::SwitchToWorkspace {
        name: workspace_name?,
        cwd,
    })
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn input_selector_switch_to_workspace_spawn_cwd_from_query(
    value: &str,
    id_param: &str,
    label_param: &str,
) -> Option<Option<WindowInputSelectorValueParam>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut cwd = None;
    let mut parsed_label = false;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        if key.eq_ignore_ascii_case("cwd") {
            if cwd.is_some() {
                return None;
            }
            cwd = Some(input_selector_callback_value_param_from_query(
                value,
                id_param,
                label_param,
            )?);
        } else if key.eq_ignore_ascii_case("label") {
            if parsed_label || value.trim().is_empty() {
                return None;
            }
            parsed_label = true;
        } else {
            return None;
        }
    }
    Some(cwd)
}

fn input_selector_callback_value_param_from_query(
    value: &str,
    id_param: &str,
    label_param: &str,
) -> Option<WindowInputSelectorValueParam> {
    let value = lua_trim_start_comments(value.trim().trim_end_matches(',').trim())?;
    let name = lua_identifier_literal_from_query(value)?;
    if !lua_static_identifier_value_rest_is_statement_end(value.get(name.len()..)?) {
        return None;
    }
    if name == id_param {
        Some(WindowInputSelectorValueParam::Id)
    } else if name == label_param {
        Some(WindowInputSelectorValueParam::Label)
    } else {
        None
    }
}

fn confirmation_options_from_query(query: &str) -> Option<WindowConfirmationOptions> {
    confirmation_options_from_query_with_static_source(None, query)
}

fn confirmation_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowConfirmationOptions> {
    let query = query.trim();
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    if let Some(rest) = strip_lua_function_call_from_query(query, "confirmation") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return confirmation_lua_table_from_query_with_static_source(static_source, rest);
        }
        if static_source.is_some()
            && let Some(options) =
                confirmation_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(options);
        }
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "confirmation=")
        && rest.trim_start().starts_with('{')
    {
        return confirmation_lua_table_from_query_with_static_source(static_source, rest);
    }

    if let Some(rest) =
        strip_query_prefix_from_any(query, &["confirmationmessage=", "confirmationmessage "])
    {
        return confirmation_action_name_options_from_query_with_static_source(
            static_source,
            rest.trim(),
        );
    }
    let rest = strip_query_prefix_from_any(query, &["confirmation=", "confirmation "])?;
    let (fields, _) = confirmation_fields_from_query_with_static_source(
        static_source,
        rest,
        ConfirmationQueryFields::default(),
    )?;

    fields.into_options()
}

fn confirmation_action_name_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowConfirmationOptions> {
    if confirmation_strip_field_key(query, "message").is_some()
        || confirmation_strip_field_key(query, "action").is_some()
        || confirmation_strip_field_key(query, "cancel").is_some()
    {
        let (fields, _) = confirmation_fields_from_query_with_static_source(
            static_source,
            query,
            ConfirmationQueryFields::default(),
        )?;
        return fields.into_options();
    }

    let fields = confirmation_field_splits(query)
        .filter_map(|(message, remaining)| {
            let fields = ConfirmationQueryFields {
                message: Some(parse_maybe_static_query_text(static_source, message)?),
                ..ConfirmationQueryFields::default()
            };
            let (fields, score) = confirmation_fields_from_query_with_static_source(
                static_source,
                remaining,
                fields,
            )?;
            Some((fields, score + 1, message.len()))
        })
        .max_by_key(|(_, score, value_len)| (*score, *value_len))?
        .0;

    fields.into_options()
}

fn confirmation_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowConfirmationOptions> {
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
    let mut fields = ConfirmationQueryFields::default();
    let mut parsed_message = false;
    let mut parsed_action = false;
    let mut parsed_cancel = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        match name.to_ascii_lowercase().as_str() {
            "message" => {
                let value =
                    modal_display_text_from_query_with_static_source(static_source, value.trim())?;
                if parsed_message || value.is_empty() {
                    return None;
                }
                fields.message = Some(value);
                parsed_message = true;
            }
            "action" => {
                if parsed_action {
                    return None;
                }
                fields.action = Some(
                    confirmation_callback_or_nested_command_from_query_with_static_source(
                        static_source,
                        value,
                    )?,
                );
                parsed_action = true;
            }
            "cancel" => {
                if parsed_cancel {
                    return None;
                }
                fields.cancel = Some(
                    confirmation_callback_or_nested_command_from_query_with_static_source(
                        static_source,
                        value,
                    )?,
                );
                parsed_cancel = true;
            }
            _ => return None,
        }
    }

    fields.into_options()
}

fn confirmation_callback_or_nested_command_from_query(value: &str) -> Option<WindowCommand> {
    confirmation_callback_or_nested_command_from_query_with_static_source(None, value)
}

fn confirmation_callback_or_nested_command_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCommand> {
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
            return confirmation_callback_or_nested_command_from_query_with_static_source(
                Some(static_source),
                &value,
            );
        }
        return confirmation_callback_or_nested_command_from_query(value);
    }
    if let Some(command) =
        lua_action_callback_perform_action_command_with_static_source(static_source, value)
    {
        return Some(command);
    }
    if lua_action_callback_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::Nop);
    }
    if let Some(static_source) = static_source
        && let Some(command) = native_key_assignment_command_from_query(Some(static_source), value)
    {
        return Some(command);
    }
    let value = parse_maybe_static_query_text(static_source, value)?;
    confirmation_nested_command_from_query(&value)
}

fn lua_action_callback_perform_action_command_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCommand> {
    lua_action_callback_perform_action_command_with_static_source_and_depth(static_source, value, 0)
}

fn lua_action_callback_perform_action_command_with_static_source_and_depth(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    depth: usize,
) -> Option<WindowCommand> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }
    if let Some(command) = lua_action_callback_perform_action_command_query(static_source, value) {
        return Some(command);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return lua_action_callback_perform_action_command_query(Some(static_source), &value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        if let Some(value) = lua_static_wezterm_action_callback_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        ) {
            return lua_action_callback_perform_action_command_query(Some(static_source), &value);
        }
        return lua_action_callback_perform_action_command_with_static_source_and_depth(
            Some(static_source),
            value,
            depth + 1,
        );
    }
    None
}

fn lua_action_callback_perform_action_command_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCommand> {
    let callback = strip_lua_function_call_from_query(value, "wezterm.action_callback")
        .or_else(|| strip_lua_function_call_from_query(value, "action_callback"))?;
    let (body, window_param, pane_param, _) =
        lua_anonymous_function_body_and_first_two_and_optional_third_params_from_query(callback)?;
    lua_action_callback_perform_action_command_body(static_source, body, window_param, pane_param)
}

fn lua_action_callback_perform_action_command_body(
    static_source: Option<LuaStaticSource<'_>>,
    body: &str,
    window_param: &str,
    pane_param: &str,
) -> Option<WindowCommand> {
    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    let mut commands = Vec::new();
    for (index, start) in starts.iter().copied().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(body.len());
        let statement = lua_trim_start_comments(body.get(start..end)?)?;
        let local_static_source = Some(LuaStaticSource {
            source: body,
            max_start: start,
        });
        if let Some(command) = lua_callback_statement_performs_action(
            local_static_source,
            static_source,
            statement,
            window_param,
            pane_param,
        )
        .or_else(|| {
            lua_callback_statement_sends_pane_input(
                local_static_source,
                static_source,
                statement,
                pane_param,
            )
        })
        .or_else(|| {
            lua_callback_statement_emits_event(
                local_static_source,
                static_source,
                statement,
                window_param,
                pane_param,
            )
        }) {
            commands.push(command);
        }
    }
    match commands.as_slice() {
        [] => None,
        [command] => Some(command.clone()),
        _ => Some(WindowCommand::Multiple(commands)),
    }
}

fn lua_callback_statement_performs_action(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    statement: &str,
    window_param: &str,
    pane_param: &str,
) -> Option<WindowCommand> {
    let action = if let Some(static_source) = static_source {
        lua_callback_statement_perform_action_query_with_static_sources(
            statement,
            window_param,
            pane_param,
            static_source,
            outer_static_source,
        )
    } else {
        lua_callback_statement_perform_action_query(statement, window_param, pane_param)
    }?;
    native_key_assignment_command_from_callback_action(
        static_source,
        outer_static_source,
        action.trim(),
    )
}

fn lua_callback_statement_perform_action_query<'a>(
    statement: &'a str,
    window_param: &str,
    pane_param: &str,
) -> Option<&'a str> {
    let parts = lua_callback_statement_perform_action_parts_query(statement)?;
    if !lua_static_identifier_expression_matches(parts.window, window_param) {
        return None;
    }
    if let Some(explicit_self) = parts.explicit_self
        && !lua_static_identifier_expression_matches(explicit_self, window_param)
    {
        return None;
    }
    if !lua_static_identifier_expression_matches(parts.pane, pane_param) {
        return None;
    }
    Some(parts.action)
}

fn lua_callback_statement_perform_action_query_with_static_sources<'a>(
    statement: &'a str,
    window_param: &str,
    pane_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<&'a str> {
    let parts = lua_callback_statement_perform_action_parts_query(statement)?;
    if !lua_callback_expression_is_window_param(
        parts.window,
        window_param,
        static_source,
        outer_static_source,
    ) {
        return None;
    }
    if let Some(explicit_self) = parts.explicit_self
        && !lua_callback_expression_is_window_param(
            explicit_self,
            window_param,
            static_source,
            outer_static_source,
        )
    {
        return None;
    }
    if !lua_callback_expression_is_pane_param(
        parts.pane,
        pane_param,
        static_source,
        outer_static_source,
    ) {
        return None;
    }
    Some(parts.action)
}

struct LuaCallbackPerformActionParts<'a> {
    window: &'a str,
    explicit_self: Option<&'a str>,
    action: &'a str,
    pane: &'a str,
}

fn lua_callback_statement_perform_action_parts_query(
    statement: &str,
) -> Option<LuaCallbackPerformActionParts<'_>> {
    let statement = lua_trim_start_comments(statement)?;
    let window = lua_identifier_literal_from_query(statement)?;
    let rest = statement.get(window.len()..)?;
    let rest = lua_trim_start_comments(rest)?;
    let (has_explicit_self, rest) = if let Some(rest) = rest.strip_prefix(':') {
        (false, rest)
    } else if let Some(rest) = rest.strip_prefix('.') {
        (true, rest)
    } else {
        return None;
    };
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with("perform_action")
        || !lua_config_assignment_field_has_boundaries(rest, 0, "perform_action")
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get("perform_action".len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    if !lua_trim_end_statement_separator(rest).trim().is_empty() {
        return None;
    }
    if has_explicit_self {
        let [explicit_self, action, pane] = arguments.as_slice() else {
            return None;
        };
        return Some(LuaCallbackPerformActionParts {
            window,
            explicit_self: Some(lua_trim_start_comments(explicit_self.trim())?),
            action: action.trim(),
            pane: lua_trim_start_comments(pane.trim())?,
        });
    }

    let [action, pane] = arguments.as_slice() else {
        return None;
    };
    Some(LuaCallbackPerformActionParts {
        window,
        explicit_self: None,
        action: action.trim(),
        pane: lua_trim_start_comments(pane.trim())?,
    })
}

fn lua_callback_expression_is_window_param(
    expression: &str,
    window_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> bool {
    lua_callback_expression_is_window_param_with_depth(
        expression,
        window_param,
        static_source,
        outer_static_source,
        0,
    )
}

fn lua_callback_expression_is_window_param_with_depth(
    expression: &str,
    window_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> bool {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return false;
    }
    if lua_static_identifier_expression_matches(expression, window_param) {
        return true;
    }
    if let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        expression,
        static_source.max_start,
    ) {
        return lua_callback_expression_is_window_param_with_depth(
            value,
            window_param,
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
        return lua_callback_expression_is_window_param_with_depth(
            value,
            window_param,
            outer_static_source,
            None,
            depth + 1,
        );
    }
    false
}

fn lua_callback_expression_is_pane_param(
    expression: &str,
    pane_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> bool {
    lua_callback_expression_is_pane_param_with_depth(
        expression,
        pane_param,
        static_source,
        outer_static_source,
        0,
    )
}

fn lua_callback_expression_is_pane_param_with_depth(
    expression: &str,
    pane_param: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> bool {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return false;
    }
    if lua_static_identifier_expression_matches(expression, pane_param) {
        return true;
    }
    if let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        expression,
        static_source.max_start,
    ) {
        return lua_callback_expression_is_pane_param_with_depth(
            value,
            pane_param,
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
        return lua_callback_expression_is_pane_param_with_depth(
            value,
            pane_param,
            outer_static_source,
            None,
            depth + 1,
        );
    }
    false
}

fn native_key_assignment_command_from_callback_action(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCommand> {
    native_key_assignment_command_from_query(static_source, value)
        .or_else(|| native_key_assignment_command_from_query(outer_static_source, value))
        .or_else(|| {
            let static_source = static_source?;
            let value = lua_static_action_assignment_value_before_offset_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )?;
            native_key_assignment_command_from_query(outer_static_source, value)
                .or_else(|| native_key_assignment_command_from_query(None, value))
        })
}

fn lua_callback_statement_sends_pane_input(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    statement: &str,
    pane_param: &str,
) -> Option<WindowCommand> {
    let statement = lua_trim_start_comments(statement)?;
    let pane = lua_identifier_literal_from_query(statement)?;
    let rest = statement.get(pane.len()..)?;
    let is_pane = if let Some(static_source) = static_source {
        lua_callback_expression_is_pane_param(pane, pane_param, static_source, outer_static_source)
    } else {
        lua_static_identifier_expression_matches(pane, pane_param)
    };
    if !is_pane {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let (command_name, command): (&str, fn(String) -> WindowCommand) = if rest
        .starts_with("send_text")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_text")
    {
        ("send_text", WindowCommand::SendString)
    } else if rest.starts_with("send_paste")
        && lua_config_assignment_field_has_boundaries(rest, 0, "send_paste")
    {
        ("send_paste", WindowCommand::SendPaste)
    } else {
        return None;
    };
    let rest = lua_trim_start_comments(rest.get(command_name.len()..)?)?;
    let text = lua_callback_statement_pane_text_argument_from_query(rest)?;
    let text = lua_static_string_value_from_expression(static_source, outer_static_source, text)?;
    Some(command(text))
}

fn lua_callback_statement_pane_text_argument_from_query(rest: &str) -> Option<&str> {
    let rest = lua_trim_start_comments(rest)?;
    if let Some(rest) = rest.strip_prefix('(') {
        let rest = lua_trim_start_comments(rest)?;
        let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        let arguments = split_lua_top_level_arguments(arguments)?;
        let [text] = arguments.as_slice() else {
            return None;
        };
        if !lua_trim_end_statement_separator(rest).trim().is_empty() {
            return None;
        }
        return Some(text);
    }

    let text = lua_top_level_statement_value_from_query(rest)?.trim();
    let (_, literal_len) = lua_inline_string_literal_value_and_len(text)?;
    lua_trim_start_comments(text.get(literal_len..)?)?
        .is_empty()
        .then_some(text)
}

fn lua_callback_statement_emits_event(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    statement: &str,
    window_param: &str,
    pane_param: &str,
) -> Option<WindowCommand> {
    let statement = lua_trim_start_comments(statement)?;
    let rest =
        lua_callback_statement_emit_call_args_query(static_source, outer_static_source, statement)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [event_name, window, pane] = arguments.as_slice() else {
        return None;
    };
    let window = lua_trim_start_comments(window.trim())?;
    let pane = lua_trim_start_comments(pane.trim())?;
    let args_match = if let Some(static_source) = static_source {
        lua_callback_expression_is_window_param(
            window,
            window_param,
            static_source,
            outer_static_source,
        ) && lua_callback_expression_is_pane_param(
            pane,
            pane_param,
            static_source,
            outer_static_source,
        )
    } else {
        lua_static_identifier_expression_matches(window, window_param)
            && lua_static_identifier_expression_matches(pane, pane_param)
    };
    if !args_match || !lua_trim_end_statement_separator(rest).trim().is_empty() {
        return None;
    }
    let name = lua_static_string_value_from_expression(
        static_source,
        outer_static_source,
        event_name.trim(),
    )?;
    (!name.is_empty()).then_some(WindowCommand::EmitEvent(WindowEmitEvent { name }))
}

fn lua_callback_statement_emit_call_args_query<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    statement: &'a str,
) -> Option<&'a str> {
    if let Some(rest) = lua_callback_statement_wezterm_emit_call_args_query(
        static_source,
        outer_static_source,
        statement,
    ) {
        return Some(rest);
    }
    if let Some(rest) = lua_callback_statement_wezterm_emit_call_args_from_require_query(
        static_source,
        outer_static_source,
        statement,
    ) {
        return Some(rest);
    }

    let alias = lua_identifier_literal_from_query(statement)?;
    let rest = statement.get(alias.len()..)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let has_local_module_alias = static_source
        .and_then(|source| {
            lua_static_wezterm_module_alias_before_offset(source.source, alias, source.max_start)
        })
        .unwrap_or(false);
    let has_outer_module_alias = outer_static_source
        .and_then(|source| {
            lua_static_wezterm_module_alias_before_offset(source.source, alias, source.max_start)
        })
        .unwrap_or(false);
    if has_local_module_alias || has_outer_module_alias {
        return lua_callback_statement_wezterm_emit_call_args_from_receiver_rest(
            static_source,
            outer_static_source,
            rest,
        );
    }
    let has_local_alias = static_source
        .and_then(|source| {
            lua_static_wezterm_emit_alias_before_offset(source.source, alias, source.max_start)
        })
        .unwrap_or(false);
    let has_outer_alias = outer_static_source
        .and_then(|source| {
            lua_static_wezterm_emit_alias_before_offset(source.source, alias, source.max_start)
        })
        .unwrap_or(false);
    if !has_local_alias && !has_outer_alias {
        return None;
    }
    lua_trim_start_comments(rest)
}

fn lua_callback_statement_wezterm_emit_call_args_query<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    statement: &'a str,
) -> Option<&'a str> {
    let rest = statement.strip_prefix("wezterm")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    lua_callback_statement_wezterm_emit_call_args_from_receiver_rest(
        static_source,
        outer_static_source,
        rest,
    )
}

fn lua_callback_statement_wezterm_emit_call_args_from_require_query<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    statement: &'a str,
) -> Option<&'a str> {
    let rest = lua_static_wezterm_require_receiver_rest_from_query(statement)?;
    lua_callback_statement_wezterm_emit_call_args_from_receiver_rest(
        static_source,
        outer_static_source,
        rest,
    )
}

fn lua_callback_statement_wezterm_emit_call_args_from_receiver_rest<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    rest: &'a str,
) -> Option<&'a str> {
    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_table_map_field_key_from_query_with_static_sources(
        static_source,
        outer_static_source,
        rest,
    )?;
    if field != "emit" {
        return None;
    }
    lua_trim_start_comments(rest)
}

fn lua_static_wezterm_emit_alias_before_offset(
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
        selected = lua_static_wezterm_emit_alias_value_from_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_emit_alias_value_from_query(
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
    lua_static_wezterm_emit_alias_receiver_rest_is_statement_end(source, max_start, rest)
}

fn lua_static_wezterm_emit_alias_receiver_rest_is_statement_end(
    source: &str,
    max_start: usize,
    rest: &str,
) -> bool {
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some((field, rest)) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource { source, max_start }),
        rest,
    ) else {
        return false;
    };
    field == "emit" && lua_static_identifier_value_rest_is_statement_end(rest)
}

#[derive(Clone, Default)]
struct ConfirmationQueryFields {
    message: Option<String>,
    action: Option<WindowCommand>,
    cancel: Option<WindowCommand>,
}

impl ConfirmationQueryFields {
    fn into_options(self) -> Option<WindowConfirmationOptions> {
        Some(WindowConfirmationOptions {
            message: self
                .message
                .unwrap_or_else(|| DEFAULT_CONFIRMATION_MESSAGE.to_owned()),
            action: Box::new(self.action?),
            cancel: self.cancel.map(Box::new),
        })
    }
}

fn confirmation_fields_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    rest: &str,
    fields: ConfirmationQueryFields,
) -> Option<(ConfirmationQueryFields, usize)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return (fields.message.is_some() && fields.action.is_some()).then_some((fields, 0));
    }

    if let Some(value) = confirmation_strip_field_key(rest, "message") {
        return confirmation_field_splits(value)
            .filter_map(|(message, remaining)| {
                if message.is_empty() || fields.message.is_some() {
                    return None;
                }
                let message_value = parse_maybe_static_query_text(static_source, message)?;
                let mut fields = fields.clone();
                fields.message = Some(message_value);
                let (fields, score) = confirmation_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    fields,
                )?;
                Some((fields, score + 1, message.len()))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(fields, score, _)| (fields, score));
    }

    if let Some(value) = confirmation_strip_field_key(rest, "action") {
        return confirmation_field_splits(value)
            .filter_map(|(action, remaining)| {
                if fields.action.is_some() {
                    return None;
                }
                let mut fields = fields.clone();
                fields.action = Some(
                    confirmation_callback_or_nested_command_from_query_with_static_source(
                        static_source,
                        action,
                    )?,
                );
                let (fields, score) = confirmation_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    fields,
                )?;
                Some((fields, score + 1, action.len()))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(fields, score, _)| (fields, score));
    }

    if let Some(value) = confirmation_strip_field_key(rest, "cancel") {
        return confirmation_field_splits(value)
            .filter_map(|(cancel, remaining)| {
                if fields.cancel.is_some() {
                    return None;
                }
                let mut fields = fields.clone();
                fields.cancel = Some(
                    confirmation_callback_or_nested_command_from_query_with_static_source(
                        static_source,
                        cancel,
                    )?,
                );
                let (fields, score) = confirmation_fields_from_query_with_static_source(
                    static_source,
                    remaining,
                    fields,
                )?;
                Some((fields, score + 1, cancel.len()))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(fields, score, _)| (fields, score));
    }

    None
}

fn confirmation_nested_command_from_query(query: &str) -> Option<WindowCommand> {
    let query = query.trim();
    if query.eq_ignore_ascii_case("nop") {
        return Some(WindowCommand::Nop);
    }
    if query.eq_ignore_ascii_case("disabledefaultassignment") {
        return Some(WindowCommand::DisableDefaultAssignment);
    }
    if let Some(title) = rename_tab_title_from_query(query) {
        return Some(WindowCommand::RenameTabTo(title));
    }
    if let Some(name) = rename_workspace_name_from_query(query) {
        return Some(WindowCommand::RenameWorkspaceTo(name));
    }
    if let Some(value) = send_string_from_query(query) {
        return Some(WindowCommand::SendString(value));
    }
    if let Some(send_key) = send_key_from_query(query) {
        return Some(WindowCommand::SendKey(send_key));
    }
    if let Some(event) = emit_event_from_query(query) {
        return Some(WindowCommand::EmitEvent(event));
    }
    if let Some(key_table) = activate_key_table_from_query(query) {
        return Some(WindowCommand::ActivateKeyTable(key_table));
    }
    if let Some(command) = key_table_stack_command_from_query(query) {
        return Some(command);
    }
    if let Some(options) = switch_workspace_options_from_query(query) {
        return Some(WindowCommand::SwitchToWorkspaceArgs(options));
    }
    if let Some(name) = switch_workspace_name_from_query(query) {
        return Some(WindowCommand::SwitchToWorkspaceName(name));
    }
    if let Some(spawn_command) = spawn_command_in_new_tab_from_query(query) {
        return Some(WindowCommand::SpawnCommandInNewTab(spawn_command));
    }
    if let Some(spawn_options) = spawn_command_options_in_new_tab_from_query(query) {
        return Some(WindowCommand::SpawnCommandOptionsInNewTab(spawn_options));
    }
    if let Some(spawn_command) = spawn_command_in_new_window_from_query(query) {
        return Some(WindowCommand::SpawnCommandInNewWindow(spawn_command));
    }
    if let Some(spawn_options) = spawn_command_options_in_new_window_from_query(query) {
        return Some(WindowCommand::SpawnCommandOptionsInNewWindow(spawn_options));
    }
    if let Some(mode) = clear_scrollback_mode_from_query(query) {
        return Some(WindowCommand::ClearScrollback(mode));
    }
    if let Some(destination) = copy_destination_command_from_query(query) {
        return Some(WindowCommand::CopyTo(destination));
    }
    if let Some(source) = paste_source_command_from_query(query) {
        return Some(WindowCommand::PasteFrom(source));
    }
    if let Some(split_pane) = split_horizontal_options_from_query(query)
        .or_else(|| split_vertical_options_from_query(query))
    {
        return Some(WindowCommand::SplitPane(split_pane));
    }
    if let Some(options) = pane_select_nested_options_from_query(query) {
        return Some(WindowCommand::PaneSelect(options));
    }
    if let Some(args) = show_launcher_args_from_query(query) {
        return Some(WindowCommand::ShowLauncherArgs(args));
    }
    if let Some(options) = char_select_options_from_query(query) {
        return Some(WindowCommand::CharSelectArgs(options));
    }
    if let Some(options) = quick_select_lua_table_from_query(query) {
        return Some(WindowCommand::QuickSelectArgs(options));
    }
    if let Some(search_query) = search_query_from_query(query) {
        return Some(WindowCommand::Search(search_query));
    }
    if quick_select_patterns_from_query(query).is_some()
        || quick_select_pattern_from_query(query).is_some()
        || quick_select_alphabet_from_query(query).is_some()
        || quick_select_label_from_query(query).is_some()
        || quick_select_action_from_query(query).is_some()
        || quick_select_scope_lines_from_query(query).is_some()
    {
        return Some(WindowCommand::QuickSelect(quick_select_options_from_query(
            query,
        )));
    }
    close_current_command_from_query(query)
        .or_else(|| command_palette_structured_query_command(query))
}

fn pane_select_nested_options_from_query(query: &str) -> Option<WindowPaneSelectOptions> {
    if let Some(options) = pane_select_options_from_query(query) {
        return Some(options);
    }
    if let Some(query) = pane_select_mode_show_pane_ids_from_query(query) {
        return Some(WindowPaneSelectOptions {
            mode: query.mode,
            show_pane_ids: true,
            alphabet: query.alphabet,
        });
    }
    if let Some(query) = pane_select_mode_alphabet_from_query(query) {
        return Some(WindowPaneSelectOptions {
            mode: query.mode,
            show_pane_ids: false,
            alphabet: Some(query.alphabet),
        });
    }
    if let Some(alphabet) = pane_select_show_pane_ids_alphabet_from_query(query)
        .or_else(|| pane_select_activate_show_pane_ids_alphabet_from_query(query))
    {
        return Some(WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::Activate,
            show_pane_ids: true,
            alphabet: Some(alphabet),
        });
    }
    pane_select_alphabet_from_query(query)
        .or_else(|| pane_select_activate_alphabet_from_query(query))
        .map(|alphabet| WindowPaneSelectOptions {
            mode: WindowPaneSelectMode::Activate,
            show_pane_ids: false,
            alphabet: Some(alphabet),
        })
}

fn multiple_commands_from_query(query: &str) -> Option<Vec<WindowCommand>> {
    if let Some(commands) = multiple_table_commands_from_query(query) {
        return Some(commands);
    }

    let rest = strip_query_prefix_from_any(query, &["multiple=", "multiple "])?;
    let commands = split_unquoted_query_semicolons(rest)
        .into_iter()
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(confirmation_nested_command_from_query)
        .collect::<Option<Vec<_>>>()?;

    (commands.len() >= 2).then_some(commands)
}

fn multiple_table_commands_from_query(query: &str) -> Option<Vec<WindowCommand>> {
    multiple_table_commands_from_query_with_parser(
        None,
        query,
        confirmation_nested_command_from_query,
    )
}

fn multiple_table_commands_from_query_with_static_source(
    static_source: LuaStaticSource<'_>,
    query: &str,
) -> Option<Vec<WindowCommand>> {
    multiple_table_commands_from_query_with_parser(Some(static_source), query, |command| {
        native_key_assignment_command_from_query(Some(static_source), command)
    })
}

fn multiple_table_commands_from_query_with_parser(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    mut command_from_query: impl FnMut(&str) -> Option<WindowCommand>,
) -> Option<Vec<WindowCommand>> {
    let query = strip_wezterm_action_prefix(query.trim()).unwrap_or(query.trim());
    let rest = strip_lua_function_call_from_query(query, "multiple")
        .or_else(|| strip_query_table_assignment_from_prefix(query, "multiple="))?;
    let rest = rest.trim();
    let resolved_rest;
    let rest = if rest.starts_with('{') {
        rest
    } else {
        let static_source = static_source?;
        resolved_rest = lua_table_insert_value_table_string_from_query(
            static_source.source,
            rest,
            static_source.max_start,
        )?;
        resolved_rest.as_str()
    };
    let table = rest.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut commands = Vec::new();
    let mut indexed_commands = BTreeMap::new();
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !commands.is_empty() || indexed_commands.contains_key(&index) {
                return None;
            }
            indexed_commands.insert(index, command_from_query(value.trim())?);
            continue;
        }
        if !indexed_commands.is_empty() {
            return None;
        }
        commands.push(command_from_query(field)?);
    }
    if !indexed_commands.is_empty() {
        commands = (1..=indexed_commands.len())
            .map(|index| indexed_commands.remove(&index))
            .collect::<Option<Vec<_>>>()?;
    }

    (commands.len() >= 2).then_some(commands)
}

fn confirmation_field_splits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut offsets = confirmation_next_field_offsets(rest);
    offsets.reverse();
    offsets.push(rest.len());
    offsets
        .into_iter()
        .map(|offset| {
            let (value, remaining) = rest.split_at(offset);
            (value.trim(), remaining.trim_start())
        })
        .filter(|(value, _)| !value.is_empty())
}

fn confirmation_strip_field_key<'a>(rest: &'a str, key: &str) -> Option<&'a str> {
    let key_prefix = rest.get(..key.len())?;
    let remaining = rest.get(key.len()..)?;
    key_prefix
        .eq_ignore_ascii_case(key)
        .then_some(remaining)
        .and_then(|remaining| {
            remaining.strip_prefix('=').or_else(|| {
                remaining
                    .starts_with(char::is_whitespace)
                    .then_some(remaining)
            })
        })
        .map(str::trim_start)
}

fn confirmation_next_field_offsets(rest: &str) -> Vec<usize> {
    let lowercase_rest = rest.to_ascii_lowercase();
    let mut offsets = [
        " message ",
        " message=",
        " action ",
        " action=",
        " cancel ",
        " cancel=",
    ]
    .into_iter()
    .flat_map(|needle| {
        lowercase_rest
            .match_indices(needle)
            .map(|(index, _)| index + 1)
    })
    .collect::<Vec<_>>();
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

fn emit_event_from_query(query: &str) -> Option<WindowEmitEvent> {
    emit_event_from_query_with_static_source(None, query)
}

fn emit_event_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowEmitEvent> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(name) = strip_lua_function_call_from_query(query, "emitevent") {
        let name = name.trim();
        if name.starts_with('{') {
            return emit_event_lua_table_from_query(static_source, name);
        }
        if static_source.is_some()
            && let Some(event) = emit_event_lua_table_from_query(static_source, name)
        {
            return Some(event);
        }
        return parse_maybe_static_query_text(static_source, name)
            .map(|name| WindowEmitEvent { name });
    }
    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "emitevent=")
        && rest.trim_start().starts_with('{')
    {
        return emit_event_lua_table_from_query(static_source, rest);
    }

    let name = strip_query_prefix_from_any(
        query,
        &["emit event=", "emit event ", "emitevent=", "emitevent "],
    )?;
    let name = strip_query_prefix_from_any(name, &["name=", "name "]).unwrap_or(name);
    parse_maybe_static_query_text(static_source, name).map(|name| WindowEmitEvent { name })
}

fn emit_event_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowEmitEvent> {
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
    let mut event = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = parse_maybe_static_query_text(static_source, value)?;
        match name.to_ascii_lowercase().as_str() {
            "name" => {
                if event.is_some() || value.is_empty() {
                    return None;
                }
                event = Some(WindowEmitEvent { name: value });
            }
            _ => return None,
        }
    }

    event
}

fn open_uri_from_query(query: &str) -> Option<String> {
    open_uri_from_query_with_static_source(None, query)
}

fn open_uri_from_query_with_static_source(
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

    let uri = strip_lua_function_call_from_query(query, "openuri").or_else(|| {
        strip_query_prefix_from_any(query, &["open uri=", "open uri ", "openuri=", "openuri "])
    })?;
    let uri = strip_query_prefix_from_any(uri, &["uri=", "uri ", "url=", "url "]).unwrap_or(uri);
    parse_maybe_static_query_text(static_source, uri)
}

fn send_string_from_query(query: &str) -> Option<String> {
    send_string_from_query_with_static_source(None, query)
}

fn send_string_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<String> {
    let indexed_query;
    let is_wezterm_action_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        is_wezterm_action_query = true;
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        is_wezterm_action_query = true;
        indexed_query = query;
        indexed_query.as_str()
    } else {
        is_wezterm_action_query = false;
        query
    };

    if let Some(value) = strip_lua_function_call_from_query(query, "sendstring") {
        let value = value.trim();
        if value.starts_with('{') {
            return send_string_lua_table_from_query_with_static_source(static_source, value);
        }
        if static_source.is_some()
            && let Some(value) =
                send_string_lua_table_from_query_with_static_source(static_source, value)
        {
            return Some(value);
        }
        return parse_maybe_static_query_text(static_source, value);
    }
    if let Some(value) = strip_query_table_assignment_from_prefix(query, "sendstring=")
        && value.trim_start().starts_with('{')
    {
        return send_string_lua_table_from_query_with_static_source(static_source, value);
    }

    let value = strip_query_prefix_from_any(
        query,
        &["send string=", "send string ", "sendstring=", "sendstring "],
    )?;
    let value = if is_wezterm_action_query {
        lua_trim_start_comments(value)?
    } else {
        value
    };
    let value = strip_query_prefix_from_any(value, &["string=", "string "]).unwrap_or(value);
    parse_maybe_static_query_text(static_source, value)
}

fn send_string_lua_table_from_query_with_static_source(
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
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut string = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = parse_maybe_static_query_text(static_source, value)?;
        match name.to_ascii_lowercase().as_str() {
            "string" => {
                if string.is_some() || value.is_empty() {
                    return None;
                }
                string = Some(value);
            }
            _ => return None,
        }
    }

    string
}

fn strip_lua_function_call_from_query<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    let query = query.trim();
    let rest = lua_function_name_rest_from_query(query, name)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('(')?)?;
    let (arguments, tail) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    lua_trim_start_comments(tail)
        .or_else(|| tail.trim_start().starts_with("--").then_some(""))?
        .is_empty()
        .then_some(arguments)
        .and_then(lua_trim_start_comments)
        .and_then(lua_trim_end_comments)
}

fn lua_function_name_rest_from_query<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    lua_dotted_identifier_rest_from_query_preserving_tail(query, name)
        .and_then(lua_trim_start_comments)
}

fn lua_dotted_identifier_rest_from_query_preserving_tail<'a>(
    query: &'a str,
    name: &str,
) -> Option<&'a str> {
    let mut query = query.trim_start();
    for (index, segment) in name.split('.').enumerate() {
        if index > 0 {
            query = lua_trim_start_comments(query)?.strip_prefix('.')?;
            query = lua_trim_start_comments(query)?;
        }
        let candidate = query.get(..segment.len())?;
        if !candidate.eq_ignore_ascii_case(segment) {
            return None;
        }
        query = query.get(segment.len()..)?;
        if query
            .chars()
            .next()
            .is_some_and(is_lua_identifier_character)
        {
            return None;
        }
    }
    Some(query)
}

fn send_key_from_query(query: &str) -> Option<WindowSendKey> {
    send_key_from_query_with_static_source(None, query)
}

fn send_key_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSendKey> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(value) = strip_lua_function_call_from_query(query, "sendkey") {
        let value = value.trim();
        if value.starts_with('{') {
            return send_key_lua_table_from_query_with_static_source(static_source, value);
        }
        if static_source.is_some()
            && let Some(send_key) =
                send_key_lua_table_from_query_with_static_source(static_source, value)
        {
            return Some(send_key);
        }
    }

    if let Some(value) = strip_query_table_assignment_from_prefix(query, "sendkey=")
        && value.trim_start().starts_with('{')
    {
        return send_key_lua_table_from_query_with_static_source(static_source, value);
    }

    let value =
        strip_query_prefix_from_any(query, &["send key=", "send key ", "sendkey=", "sendkey "])?;
    if let Some(send_key) = send_key_fields_from_query_with_static_source(static_source, value) {
        return Some(send_key);
    }
    let value = parse_maybe_static_query_text(static_source, value)?;
    let mut modifiers = ModifiersState::empty();
    let mut leader_required = false;
    let mut key = None;

    for token in value.split('+').map(str::trim) {
        if token.is_empty() {
            return None;
        }

        if token != "|" && token.contains('|') {
            for modifier in token.split('|').map(str::trim) {
                if !window_key_assignment_modifier_matches(
                    modifier,
                    &mut modifiers,
                    &mut leader_required,
                ) || leader_required
                {
                    return None;
                }
            }
        } else if window_key_assignment_modifier_matches(
            token,
            &mut modifiers,
            &mut leader_required,
        ) {
            if leader_required {
                return None;
            }
        } else {
            if key.is_some() {
                return None;
            }
            key = Some(send_key_key_from_query(token)?);
        }
    }

    Some(WindowSendKey {
        key: key?,
        modifiers,
    })
}

fn send_key_fields_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowSendKey> {
    let tokens = command_palette_query_words(value)?;
    let mut modifiers = ModifiersState::empty();
    let mut leader_required = false;
    let mut key = None;
    let mut parsed_key = false;
    let mut parsed_mods = false;

    for token in tokens {
        if let Some(value) = query_assignment_value_from_token(&token, &["key"]) {
            if parsed_key {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            key = Some(send_key_key_from_query(&value)?);
            parsed_key = true;
            continue;
        }

        if let Some(value) = query_assignment_value_from_token(&token, &["mods"]) {
            if parsed_mods {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            for modifier in value.split(['+', '|']).map(str::trim) {
                if !window_key_assignment_modifier_matches(
                    modifier,
                    &mut modifiers,
                    &mut leader_required,
                ) || leader_required
                {
                    return None;
                }
            }
            parsed_mods = true;
            continue;
        }

        return None;
    }

    parsed_key.then_some(WindowSendKey {
        key: key?,
        modifiers,
    })
}

fn send_key_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowSendKey> {
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
    let mut modifiers = ModifiersState::empty();
    let mut leader_required = false;
    let mut key = None;
    let mut parsed_key = false;
    let mut parsed_mods = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = parse_maybe_static_query_text(static_source, value)?;
        match name.to_ascii_lowercase().as_str() {
            "key" => {
                if parsed_key {
                    return None;
                }
                key = Some(send_key_key_from_query(&value)?);
                parsed_key = true;
            }
            "mods" => {
                if parsed_mods {
                    return None;
                }
                for modifier in value.split(['+', '|']).map(str::trim) {
                    if !window_key_assignment_modifier_matches(
                        modifier,
                        &mut modifiers,
                        &mut leader_required,
                    ) || leader_required
                    {
                        return None;
                    }
                }
                parsed_mods = true;
            }
            _ => return None,
        }
    }

    parsed_key.then_some(WindowSendKey {
        key: key?,
        modifiers,
    })
}

fn send_key_key_from_query(token: &str) -> Option<Key> {
    send_key_named_key_from_query(token).or_else(|| send_key_character_from_query(token))
}

fn send_key_named_key_from_query(token: &str) -> Option<Key> {
    let token = token
        .chars()
        .filter(|character| *character != '_' && *character != '-')
        .collect::<String>()
        .to_ascii_uppercase();

    if let Some(number) = token
        .strip_prefix('F')
        .and_then(|number| number.parse::<u8>().ok())
    {
        return send_key_function_key_from_query(number).map(Key::Named);
    }

    let named = match token.as_str() {
        "ENTER" | "RETURN" => NamedKey::Enter,
        "ESC" | "ESCAPE" => NamedKey::Escape,
        "TAB" => NamedKey::Tab,
        "SPACE" => return Some(Key::Character(" ".into())),
        "BACKSPACE" => NamedKey::Backspace,
        "INSERT" | "INS" => NamedKey::Insert,
        "DELETE" | "DEL" => NamedKey::Delete,
        "HOME" => NamedKey::Home,
        "END" => NamedKey::End,
        "PAGEUP" => NamedKey::PageUp,
        "PAGEDOWN" => NamedKey::PageDown,
        "LEFTARROW" | "ARROWLEFT" => NamedKey::ArrowLeft,
        "RIGHTARROW" | "ARROWRIGHT" => NamedKey::ArrowRight,
        "UPARROW" | "ARROWUP" => NamedKey::ArrowUp,
        "DOWNARROW" | "ARROWDOWN" => NamedKey::ArrowDown,
        "CAPSLOCK" => NamedKey::CapsLock,
        "SCROLLLOCK" => NamedKey::ScrollLock,
        "NUMLOCK" => NamedKey::NumLock,
        "PRINTSCREEN" => NamedKey::PrintScreen,
        "PAUSE" => NamedKey::Pause,
        "MENU" | "CONTEXTMENU" => NamedKey::ContextMenu,
        "MEDIAPLAY" => NamedKey::MediaPlay,
        "MEDIAPAUSE" => NamedKey::MediaPause,
        "MEDIAPLAYPAUSE" => NamedKey::MediaPlayPause,
        "MEDIANEXTTRACK" | "MEDIATRACKNEXT" => NamedKey::MediaTrackNext,
        "MEDIAPREVTRACK" | "MEDIAPREVIOUSTRACK" | "MEDIATRACKPREVIOUS" => {
            NamedKey::MediaTrackPrevious
        }
        "MEDIAREWIND" => NamedKey::MediaRewind,
        "MEDIASTOP" => NamedKey::MediaStop,
        "MEDIAFASTFORWARD" => NamedKey::MediaFastForward,
        "MEDIARECORD" => NamedKey::MediaRecord,
        "VOLUMEDOWN" | "AUDIOVOLUMEDOWN" => NamedKey::AudioVolumeDown,
        "VOLUMEUP" | "AUDIOVOLUMEUP" => NamedKey::AudioVolumeUp,
        "VOLUMEMUTE" | "AUDIOVOLUMEMUTE" => NamedKey::AudioVolumeMute,
        "BROWSERBACK" => NamedKey::BrowserBack,
        "BROWSERFORWARD" => NamedKey::BrowserForward,
        "BROWSERREFRESH" => NamedKey::BrowserRefresh,
        "BROWSERSTOP" => NamedKey::BrowserStop,
        "BROWSERSEARCH" => NamedKey::BrowserSearch,
        "BROWSERFAVORITES" => NamedKey::BrowserFavorites,
        "BROWSERHOME" => NamedKey::BrowserHome,
        _ => return None,
    };

    Some(Key::Named(named))
}

fn send_key_function_key_from_query(number: u8) -> Option<NamedKey> {
    match number {
        1 => Some(NamedKey::F1),
        2 => Some(NamedKey::F2),
        3 => Some(NamedKey::F3),
        4 => Some(NamedKey::F4),
        5 => Some(NamedKey::F5),
        6 => Some(NamedKey::F6),
        7 => Some(NamedKey::F7),
        8 => Some(NamedKey::F8),
        9 => Some(NamedKey::F9),
        10 => Some(NamedKey::F10),
        11 => Some(NamedKey::F11),
        12 => Some(NamedKey::F12),
        13 => Some(NamedKey::F13),
        14 => Some(NamedKey::F14),
        15 => Some(NamedKey::F15),
        16 => Some(NamedKey::F16),
        17 => Some(NamedKey::F17),
        18 => Some(NamedKey::F18),
        19 => Some(NamedKey::F19),
        20 => Some(NamedKey::F20),
        21 => Some(NamedKey::F21),
        22 => Some(NamedKey::F22),
        23 => Some(NamedKey::F23),
        24 => Some(NamedKey::F24),
        25 => Some(NamedKey::F25),
        26 => Some(NamedKey::F26),
        27 => Some(NamedKey::F27),
        28 => Some(NamedKey::F28),
        29 => Some(NamedKey::F29),
        30 => Some(NamedKey::F30),
        31 => Some(NamedKey::F31),
        32 => Some(NamedKey::F32),
        33 => Some(NamedKey::F33),
        34 => Some(NamedKey::F34),
        35 => Some(NamedKey::F35),
        _ => None,
    }
}

fn send_key_character_from_query(token: &str) -> Option<Key> {
    let mut chars = token.chars();
    let character = chars.next()?;
    chars.next().is_none().then(|| {
        let character = if character.is_ascii() {
            character.to_ascii_lowercase().to_string()
        } else {
            character.to_string()
        };
        Key::Character(character.into())
    })
}

fn key_table_stack_command_from_query(query: &str) -> Option<WindowCommand> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };
    let query = strip_zero_arg_lua_function_call_from_query(query).unwrap_or(query);
    let action_name = normalized_action_name_query(query);
    match action_name.as_str() {
        "popkeytable" => Some(WindowCommand::PopKeyTable),
        "clearkeytablestack" => Some(WindowCommand::ClearKeyTableStack),
        _ => None,
    }
}

fn activate_key_table_from_query(query: &str) -> Option<WindowActivateKeyTable> {
    activate_key_table_from_query_with_static_source(None, query)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn activate_key_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowActivateKeyTable> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(rest) = strip_lua_function_call_from_query(query, "activatekeytable") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return activate_key_table_lua_table_from_query(static_source, rest);
        }
        if static_source.is_some()
            && let Some(key_table) = activate_key_table_lua_table_from_query(static_source, rest)
        {
            return Some(key_table);
        }
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "activatekeytable=")
        && rest.trim_start().starts_with('{')
    {
        return activate_key_table_lua_table_from_query(static_source, rest);
    }

    let rest = strip_query_prefix_from_any(
        query,
        &[
            "activate key table=",
            "activate key table ",
            "activatekeytable=",
            "activatekeytable ",
        ],
    )?;
    let mut tokens = command_palette_query_words(rest)?;
    let name = parse_non_empty_query_text(tokens.first().map(String::as_str)?)?.to_owned();
    tokens.remove(0);

    let mut key_table = WindowActivateKeyTable {
        name,
        timeout_milliseconds: None,
        one_shot: true,
        replace_current: false,
        until_unknown: false,
        prevent_fallback: false,
    };
    let mut index = 0;
    let mut parsed_timeout = false;
    let mut parsed_one_shot = false;
    let mut parsed_replace_current = false;
    let mut parsed_until_unknown = false;
    let mut parsed_prevent_fallback = false;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        if let Some(value) = query_assignment_value_from_token(
            token,
            &["timeout", "timeout_milliseconds", "timeout-milliseconds"],
        ) {
            if parsed_timeout {
                return None;
            }
            key_table.timeout_milliseconds = Some(value.parse().ok()?);
            parsed_timeout = true;
            index += 1;
            continue;
        }
        if let Some(value) = query_assignment_value_from_token(token, &["one_shot", "one-shot"]) {
            if parsed_one_shot {
                return None;
            }
            key_table.one_shot = bool_from_query(value)?;
            parsed_one_shot = true;
            index += 1;
            continue;
        }
        if let Some(value) =
            query_assignment_value_from_token(token, &["replace_current", "replace-current"])
        {
            if parsed_replace_current {
                return None;
            }
            key_table.replace_current = bool_from_query(value)?;
            parsed_replace_current = true;
            index += 1;
            continue;
        }
        if let Some(value) =
            query_assignment_value_from_token(token, &["until_unknown", "until-unknown"])
        {
            if parsed_until_unknown {
                return None;
            }
            key_table.until_unknown = bool_from_query(value)?;
            parsed_until_unknown = true;
            index += 1;
            continue;
        }
        if let Some(value) =
            query_assignment_value_from_token(token, &["prevent_fallback", "prevent-fallback"])
        {
            if parsed_prevent_fallback {
                return None;
            }
            key_table.prevent_fallback = bool_from_query(value)?;
            parsed_prevent_fallback = true;
            index += 1;
            continue;
        }
        let token_key = tokens[index].to_ascii_lowercase();
        match token_key.as_str() {
            "timeout"
                if tokens.get(index + 1).is_some_and(|token| {
                    token.eq_ignore_ascii_case("milliseconds")
                        || starts_with_ascii_case_insensitive(token, "milliseconds=")
                }) =>
            {
                if parsed_timeout {
                    return None;
                }
                let value = if let Some(value) = query_assignment_value_from_token(
                    tokens.get(index + 1).map(String::as_str)?,
                    &["milliseconds"],
                ) {
                    index += 2;
                    value
                } else {
                    index += 3;
                    parse_single_query_value(tokens.get(index - 1).map(String::as_str)?)?
                };
                key_table.timeout_milliseconds = Some(value.parse().ok()?);
                parsed_timeout = true;
            }
            "timeout" | "timeout_milliseconds" | "timeout-milliseconds" => {
                if parsed_timeout {
                    return None;
                }
                let value = parse_single_query_value(tokens.get(index + 1).map(String::as_str)?)?;
                key_table.timeout_milliseconds = Some(value.parse().ok()?);
                parsed_timeout = true;
                index += 2;
            }
            "one_shot" | "one-shot" => {
                if parsed_one_shot {
                    return None;
                }
                key_table.one_shot = bool_from_query(parse_single_query_value(
                    tokens.get(index + 1).map(String::as_str)?,
                )?)?;
                parsed_one_shot = true;
                index += 2;
            }
            "one"
                if tokens.get(index + 1).is_some_and(|token| {
                    token.eq_ignore_ascii_case("shot")
                        || starts_with_ascii_case_insensitive(token, "shot=")
                }) =>
            {
                if parsed_one_shot {
                    return None;
                }
                let value = if let Some(value) = query_assignment_value_from_token(
                    tokens.get(index + 1).map(String::as_str)?,
                    &["shot"],
                ) {
                    index += 2;
                    value
                } else {
                    index += 3;
                    parse_single_query_value(tokens.get(index - 1).map(String::as_str)?)?
                };
                key_table.one_shot = bool_from_query(value)?;
                parsed_one_shot = true;
            }
            "replace_current" | "replace-current" => {
                if parsed_replace_current {
                    return None;
                }
                key_table.replace_current = bool_from_query(parse_single_query_value(
                    tokens.get(index + 1).map(String::as_str)?,
                )?)?;
                parsed_replace_current = true;
                index += 2;
            }
            "replace"
                if tokens.get(index + 1).is_some_and(|token| {
                    token.eq_ignore_ascii_case("current")
                        || starts_with_ascii_case_insensitive(token, "current=")
                }) =>
            {
                if parsed_replace_current {
                    return None;
                }
                let value = if let Some(value) = query_assignment_value_from_token(
                    tokens.get(index + 1).map(String::as_str)?,
                    &["current"],
                ) {
                    index += 2;
                    value
                } else {
                    index += 3;
                    parse_single_query_value(tokens.get(index - 1).map(String::as_str)?)?
                };
                key_table.replace_current = bool_from_query(value)?;
                parsed_replace_current = true;
            }
            "until_unknown" | "until-unknown" => {
                if parsed_until_unknown {
                    return None;
                }
                key_table.until_unknown = bool_from_query(parse_single_query_value(
                    tokens.get(index + 1).map(String::as_str)?,
                )?)?;
                parsed_until_unknown = true;
                index += 2;
            }
            "until"
                if tokens.get(index + 1).is_some_and(|token| {
                    token.eq_ignore_ascii_case("unknown")
                        || starts_with_ascii_case_insensitive(token, "unknown=")
                }) =>
            {
                if parsed_until_unknown {
                    return None;
                }
                let value = if let Some(value) = query_assignment_value_from_token(
                    tokens.get(index + 1).map(String::as_str)?,
                    &["unknown"],
                ) {
                    index += 2;
                    value
                } else {
                    index += 3;
                    parse_single_query_value(tokens.get(index - 1).map(String::as_str)?)?
                };
                key_table.until_unknown = bool_from_query(value)?;
                parsed_until_unknown = true;
            }
            "prevent_fallback" | "prevent-fallback" => {
                if parsed_prevent_fallback {
                    return None;
                }
                key_table.prevent_fallback = bool_from_query(parse_single_query_value(
                    tokens.get(index + 1).map(String::as_str)?,
                )?)?;
                parsed_prevent_fallback = true;
                index += 2;
            }
            "prevent"
                if tokens.get(index + 1).is_some_and(|token| {
                    token.eq_ignore_ascii_case("fallback")
                        || starts_with_ascii_case_insensitive(token, "fallback=")
                }) =>
            {
                if parsed_prevent_fallback {
                    return None;
                }
                let value = if let Some(value) = query_assignment_value_from_token(
                    tokens.get(index + 1).map(String::as_str)?,
                    &["fallback"],
                ) {
                    index += 2;
                    value
                } else {
                    index += 3;
                    parse_single_query_value(tokens.get(index - 1).map(String::as_str)?)?
                };
                key_table.prevent_fallback = bool_from_query(value)?;
                parsed_prevent_fallback = true;
            }
            _ => return None,
        }
    }

    Some(key_table)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn activate_key_table_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowActivateKeyTable> {
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
    let mut key_table = WindowActivateKeyTable {
        name: String::new(),
        timeout_milliseconds: None,
        one_shot: true,
        replace_current: false,
        until_unknown: false,
        prevent_fallback: false,
    };
    let mut parsed_name = false;
    let mut parsed_timeout = false;
    let mut parsed_one_shot = false;
    let mut parsed_replace_current = false;
    let mut parsed_until_unknown = false;
    let mut parsed_prevent_fallback = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "name" => {
                if parsed_name || value.is_empty() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                key_table.name = value;
                parsed_name = true;
            }
            "timeout" | "timeout_milliseconds" | "timeout-milliseconds" => {
                if parsed_timeout {
                    return None;
                }
                let value = if let Some(static_source) = static_source {
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
                };
                key_table.timeout_milliseconds = Some(value.parse().ok()?);
                parsed_timeout = true;
            }
            "one_shot" | "one-shot" => {
                if parsed_one_shot {
                    return None;
                }
                let value = if let Some(static_source) = static_source {
                    lua_static_bool_assignment_value_before_offset_from_query(
                        static_source.source,
                        value,
                        static_source.max_start,
                    )
                    .map(str::to_owned)
                    .or_else(|| parse_maybe_quoted_query_text(value))?
                } else {
                    parse_maybe_quoted_query_text(value)?
                };
                key_table.one_shot = bool_from_query(&value)?;
                parsed_one_shot = true;
            }
            "replace_current" | "replace-current" => {
                if parsed_replace_current {
                    return None;
                }
                let value = if let Some(static_source) = static_source {
                    lua_static_bool_assignment_value_before_offset_from_query(
                        static_source.source,
                        value,
                        static_source.max_start,
                    )
                    .map(str::to_owned)
                    .or_else(|| parse_maybe_quoted_query_text(value))?
                } else {
                    parse_maybe_quoted_query_text(value)?
                };
                key_table.replace_current = bool_from_query(&value)?;
                parsed_replace_current = true;
            }
            "until_unknown" | "until-unknown" => {
                if parsed_until_unknown {
                    return None;
                }
                let value = if let Some(static_source) = static_source {
                    lua_static_bool_assignment_value_before_offset_from_query(
                        static_source.source,
                        value,
                        static_source.max_start,
                    )
                    .map(str::to_owned)
                    .or_else(|| parse_maybe_quoted_query_text(value))?
                } else {
                    parse_maybe_quoted_query_text(value)?
                };
                key_table.until_unknown = bool_from_query(&value)?;
                parsed_until_unknown = true;
            }
            "prevent_fallback" | "prevent-fallback" => {
                if parsed_prevent_fallback {
                    return None;
                }
                let value = if let Some(static_source) = static_source {
                    lua_static_bool_assignment_value_before_offset_from_query(
                        static_source.source,
                        value,
                        static_source.max_start,
                    )
                    .map(str::to_owned)
                    .or_else(|| parse_maybe_quoted_query_text(value))?
                } else {
                    parse_maybe_quoted_query_text(value)?
                };
                key_table.prevent_fallback = bool_from_query(&value)?;
                parsed_prevent_fallback = true;
            }
            _ => return None,
        }
    }

    parsed_name.then_some(key_table)
}

fn query_assignment_value_from_token<'a>(token: &'a str, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        let prefix = token.get(..key.len())?;
        let rest = token.get(key.len()..)?;
        prefix
            .eq_ignore_ascii_case(key)
            .then_some(rest)?
            .strip_prefix('=')
            .and_then(parse_single_query_value)
    })
}

fn query_text_assignment_value_from_token<'a>(token: &'a str, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        let prefix = token.get(..key.len())?;
        let rest = token.get(key.len()..)?;
        prefix
            .eq_ignore_ascii_case(key)
            .then_some(rest)?
            .strip_prefix('=')
            .and_then(parse_non_empty_query_text)
    })
}

fn paste_source_from_query(source: &str) -> Option<WindowPasteSource> {
    paste_source_from_query_with_static_source(None, source)
}

fn paste_source_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
) -> Option<WindowPasteSource> {
    let source = parse_maybe_static_query_text(static_source, source)?;
    let normalized = source
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "clipboard" => Some(WindowPasteSource::Clipboard),
        "primary" | "primaryselection" => Some(WindowPasteSource::PrimarySelection),
        _ => None,
    }
}

fn show_launcher_args_from_query(query: &str) -> Option<WindowShowLauncherArgs> {
    show_launcher_args_from_query_with_static_source(None, query)
}

fn show_launcher_args_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowShowLauncherArgs> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    if let Some(rest) = strip_lua_function_call_from_query(query, "showlauncherargs") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return show_launcher_args_lua_table_from_query_with_static_source(static_source, rest);
        }
        if static_source.is_some()
            && let Some(args) =
                show_launcher_args_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(args);
        }
    }

    let args = strip_query_prefix_from_any(
        query,
        &[
            "show launcher args=",
            "show launcher args ",
            "show launcher=",
            "show launcher ",
            "showlauncherargs=",
            "showlauncherargs ",
            "showlauncher=",
            "showlauncher ",
        ],
    )?;
    let args = args.trim();
    if args.starts_with('{') {
        return show_launcher_args_lua_table_from_query_with_static_source(static_source, args);
    }
    if static_source.is_some() {
        return show_launcher_args_from_query_with_static_source(None, query);
    }
    let (flags, fields) = show_launcher_args_from_query_flags_first(args).or_else(|| {
        let fields = show_launcher_fields_from_query_rest(args)?;
        let flags = fields.flags.clone()?;
        Some((flags, fields))
    })?;

    Some(WindowShowLauncherArgs {
        flags,
        title: fields.title,
        alphabet: fields.alphabet,
        help_text: fields.help_text,
        fuzzy_help_text: fields.fuzzy_help_text,
    })
}

fn show_launcher_args_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowShowLauncherArgs> {
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
    let mut fields = WindowShowLauncherQueryFields::default();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = parse_maybe_static_query_text(static_source, value.trim())?;

        match normalized_show_launcher_lua_field(&name).as_str() {
            "flags" => {
                if fields.flags.is_some() {
                    return None;
                }
                fields.flags = Some(WindowShowLauncherFlags::from_pipe_separated(&value)?);
            }
            "alphabet" => {
                if fields.alphabet.is_some() {
                    return None;
                }
                fields.alphabet = Some(value);
            }
            "title" => {
                if fields.title.is_some() {
                    return None;
                }
                fields.title = Some(value);
            }
            "helptext" => {
                if fields.help_text.is_some() {
                    return None;
                }
                fields.help_text = Some(value);
            }
            "fuzzyhelptext" => {
                if fields.fuzzy_help_text.is_some() {
                    return None;
                }
                fields.fuzzy_help_text = Some(value);
            }
            _ => return None,
        }
    }

    Some(WindowShowLauncherArgs {
        flags: fields.flags?,
        title: fields.title,
        alphabet: fields.alphabet,
        help_text: fields.help_text,
        fuzzy_help_text: fields.fuzzy_help_text,
    })
}

fn normalized_show_launcher_lua_field(field: &str) -> String {
    field
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn show_launcher_args_from_query_flags_first(
    args: &str,
) -> Option<(WindowShowLauncherFlags, WindowShowLauncherQueryFields)> {
    let (flags, rest) = if let Some(value) = show_launcher_strip_field_key(args, "flags") {
        let (flags, rest) = show_launcher_one_word_splits(value).next()?;
        (parse_maybe_quoted_query_text(flags)?, rest)
    } else {
        let (flags, rest) = args
            .split_once(char::is_whitespace)
            .map_or((args, ""), |(flags, rest)| (flags, rest.trim()));
        (flags.to_owned(), rest)
    };
    let flags = WindowShowLauncherFlags::from_pipe_separated(&flags)?;
    let fields = show_launcher_fields_from_query_rest(rest)?;
    let flags = fields.flags.clone().unwrap_or(flags);

    Some((flags, fields))
}

#[derive(Clone, Default)]
struct WindowShowLauncherQueryFields {
    flags: Option<WindowShowLauncherFlags>,
    alphabet: Option<String>,
    title: Option<String>,
    help_text: Option<String>,
    fuzzy_help_text: Option<String>,
}

fn show_launcher_fields_from_query_rest(rest: &str) -> Option<WindowShowLauncherQueryFields> {
    show_launcher_fields_from_query(rest, WindowShowLauncherQueryFields::default())
        .map(|(fields, _)| fields)
}

fn show_launcher_fields_from_query(
    rest: &str,
    fields: WindowShowLauncherQueryFields,
) -> Option<(WindowShowLauncherQueryFields, usize)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Some((fields, 0));
    }

    if let Some(value) = show_launcher_strip_field_key(rest, "flags") {
        return show_launcher_one_word_splits(value)
            .filter_map(|(flags, remaining)| {
                if fields.flags.is_some() {
                    return None;
                }
                let flags = parse_maybe_quoted_query_text(flags)?;
                let flags = WindowShowLauncherFlags::from_pipe_separated(&flags)?;
                let mut fields = fields.clone();
                fields.flags = Some(flags);
                let (fields, score) = show_launcher_fields_from_query(remaining, fields)?;
                Some((fields, score + 1))
            })
            .max_by_key(|(_, score)| *score);
    }

    if let Some(value) = show_launcher_strip_field_key(rest, "alphabet") {
        return show_launcher_one_word_splits(value)
            .filter_map(|(alphabet, remaining)| {
                if fields.alphabet.is_some() {
                    return None;
                }
                let alphabet = parse_maybe_quoted_query_text(alphabet)?;
                let mut fields = fields.clone();
                let alphabet_len = alphabet.len();
                fields.alphabet = Some(alphabet);
                let (fields, score) = show_launcher_fields_from_query(remaining, fields)?;
                Some((fields, score + 1, alphabet_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(fields, score, _)| (fields, score));
    }

    if let Some(value) = show_launcher_strip_field_key(rest, "title") {
        return show_launcher_text_splits(value)
            .filter_map(|(title, remaining)| {
                if fields.title.is_some() {
                    return None;
                }
                let title = parse_maybe_quoted_query_text(title)?;
                let mut fields = fields.clone();
                let title_len = title.len();
                fields.title = Some(title);
                let (fields, score) = show_launcher_fields_from_query(remaining, fields)?;
                Some((fields, score + 1, title_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(fields, score, _)| (fields, score));
    }

    if let Some(value) =
        show_launcher_strip_field_key_from_any(rest, &["help_text", "help text", "help-text"])
    {
        return show_launcher_text_splits(value)
            .filter_map(|(help_text, remaining)| {
                if fields.help_text.is_some() {
                    return None;
                }
                let help_text = parse_maybe_quoted_query_text(help_text)?;
                let mut fields = fields.clone();
                let help_text_len = help_text.len();
                fields.help_text = Some(help_text);
                let (fields, score) = show_launcher_fields_from_query(remaining, fields)?;
                Some((fields, score + 1, help_text_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(fields, score, _)| (fields, score));
    }

    if let Some(value) = show_launcher_strip_field_key_from_any(
        rest,
        &["fuzzy_help_text", "fuzzy help text", "fuzzy-help-text"],
    ) {
        return show_launcher_text_splits(value)
            .filter_map(|(fuzzy_help_text, remaining)| {
                if fields.fuzzy_help_text.is_some() {
                    return None;
                }
                let fuzzy_help_text = parse_maybe_quoted_query_text(fuzzy_help_text)?;
                let mut fields = fields.clone();
                let fuzzy_help_text_len = fuzzy_help_text.len();
                fields.fuzzy_help_text = Some(fuzzy_help_text);
                let (fields, score) = show_launcher_fields_from_query(remaining, fields)?;
                Some((fields, score + 1, fuzzy_help_text_len))
            })
            .max_by_key(|(_, score, value_len)| (*score, *value_len))
            .map(|(fields, score, _)| (fields, score));
    }

    None
}

fn show_launcher_one_word_splits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let (value, remaining) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, ""), |(value, remaining)| {
            (value, remaining.trim_start())
        });
    std::iter::once((value, remaining))
}

fn show_launcher_text_splits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut offsets = show_launcher_next_field_offsets(rest);
    offsets.reverse();
    offsets.push(rest.len());
    offsets
        .into_iter()
        .map(|offset| {
            let (value, remaining) = rest.split_at(offset);
            (value.trim(), remaining.trim_start())
        })
        .filter(|(value, _)| !value.is_empty())
}

fn show_launcher_strip_field_key<'a>(rest: &'a str, key: &str) -> Option<&'a str> {
    let key_prefix = rest.get(..key.len())?;
    let remaining = rest.get(key.len()..)?;
    key_prefix
        .eq_ignore_ascii_case(key)
        .then_some(remaining)
        .and_then(|remaining| {
            remaining.strip_prefix('=').or_else(|| {
                remaining
                    .starts_with(char::is_whitespace)
                    .then_some(remaining)
            })
        })
        .map(str::trim_start)
}

fn show_launcher_strip_field_key_from_any<'a>(rest: &'a str, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| show_launcher_strip_field_key(rest, key))
}

fn show_launcher_next_field_offsets(rest: &str) -> Vec<usize> {
    let lowercase_rest = rest.to_ascii_lowercase();
    let mut offsets = [
        " flags ",
        " flags=",
        " alphabet ",
        " alphabet=",
        " title ",
        " title=",
        " help_text ",
        " help_text=",
        " help text ",
        " help text=",
        " help-text ",
        " help-text=",
        " fuzzy_help_text ",
        " fuzzy_help_text=",
        " fuzzy help text ",
        " fuzzy help text=",
        " fuzzy-help-text ",
        " fuzzy-help-text=",
    ]
    .into_iter()
    .flat_map(|needle| {
        lowercase_rest
            .match_indices(needle)
            .map(|(index, _)| index + 1)
    })
    .collect::<Vec<_>>();
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

fn rename_tab_title_from_query(query: &str) -> Option<String> {
    rename_tab_title_from_query_with_static_source(None, query)
}

fn rename_tab_title_from_query_with_static_source(
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

    if let Some(title) = strip_lua_function_call_from_query(query, "renametab") {
        return parse_maybe_static_query_text(static_source, title);
    }

    let title = strip_query_prefix_from_any(
        query,
        &["rename tab=", "rename tab ", "renametab=", "renametab "],
    )?;
    let title = strip_query_prefix_from_any(title, &["title=", "title "]).unwrap_or(title);
    parse_maybe_static_query_text(static_source, title)
}

fn rename_workspace_name_from_query(query: &str) -> Option<String> {
    rename_workspace_name_from_query_with_static_source(None, query)
}

fn rename_workspace_name_from_query_with_static_source(
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

    if let Some(name) = strip_lua_function_call_from_query(query, "renameworkspace") {
        return parse_maybe_static_query_text(static_source, name);
    }

    let name = strip_query_prefix_from_any(
        query,
        &[
            "rename workspace=",
            "rename workspace ",
            "renameworkspace=",
            "renameworkspace ",
        ],
    )?;
    let name = strip_query_prefix_from_any(name, &["name=", "name "]).unwrap_or(name);
    parse_maybe_static_query_text(static_source, name)
}

fn switch_workspace_name_from_query(query: &str) -> Option<String> {
    strip_query_prefix_from_any(
        query,
        &[
            "switch workspace=",
            "switch workspace ",
            "switch to workspace=",
            "switch to workspace ",
            "switchtoworkspace=",
            "switchtoworkspace ",
        ],
    )
    .map(str::trim)
    .filter(|name| !name.is_empty())
    .map(str::to_owned)
}

fn switch_workspace_relative_from_query(query: &str) -> Option<isize> {
    switch_workspace_relative_from_query_with_static_source(None, query)
}

fn switch_workspace_relative_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(offset) = strip_lua_function_call_from_query(query, "switchworkspacerelative")
        .and_then(|offset| parse_maybe_static_query_isize(static_source, offset))
    {
        return Some(offset);
    }

    let offset = strip_query_prefix_from_any(
        query,
        &[
            "switch workspace relative=",
            "switch workspace relative ",
            "switch to workspace relative=",
            "switch to workspace relative ",
            "switchworkspacerelative=",
            "switchworkspacerelative ",
        ],
    )
    .and_then(parse_single_query_value)?;
    strip_query_prefix_from_any(offset, &["offset=", "offset ", "amount=", "amount "])
        .or(Some(offset))
        .and_then(|offset| parse_maybe_static_query_isize(static_source, offset))
}

fn switch_workspace_options_from_query(query: &str) -> Option<WindowSwitchToWorkspaceOptions> {
    switch_workspace_options_from_query_with_static_source(None, query)
}

fn switch_workspace_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSwitchToWorkspaceOptions> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);

    if let Some(options) =
        switch_workspace_lua_table_options_from_query_with_static_source(static_source, query)
    {
        return Some(options);
    }

    let rest = strip_query_prefix_from_any(
        query,
        &[
            "switch workspace=",
            "switch workspace ",
            "switch to workspace=",
            "switch to workspace ",
            "switchtoworkspace=",
            "switchtoworkspace ",
        ],
    )?;
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    if rest.starts_with('{') {
        return switch_workspace_options_lua_table_from_query_with_static_source(
            static_source,
            rest,
        );
    }

    let (name, command, command_options) = match split_unquoted_query_marker(rest, " spawn ") {
        _ if starts_with_ascii_case_insensitive(rest, "spawn ") => {
            let command = strip_query_prefix_from_any(rest, &["spawn "])?;
            if command.is_empty() {
                return None;
            }
            let (command, command_options) = switch_workspace_spawn_from_query(command);
            (None, command, command_options)
        }
        Some((name, command)) => {
            let name = switch_workspace_query_name(name)?;
            let command = command.trim();
            if command.is_empty() {
                return None;
            }
            let (command, command_options) = switch_workspace_spawn_from_query(command);
            (Some(name), command, command_options)
        }
        None => (Some(switch_workspace_query_name(rest)?), None, None),
    };

    Some(WindowSwitchToWorkspaceOptions {
        name,
        command,
        command_options,
    })
}

fn switch_workspace_lua_table_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSwitchToWorkspaceOptions> {
    if let Some(rest) = strip_lua_function_call_from_query(query, "switchtoworkspace") {
        return switch_workspace_options_lua_table_from_query_with_static_source(
            static_source,
            rest,
        );
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "switchtoworkspace=") {
        return switch_workspace_options_lua_table_from_query_with_static_source(
            static_source,
            rest,
        );
    }

    None
}

fn switch_workspace_options_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowSwitchToWorkspaceOptions> {
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
    let mut name = None;
    let mut command = None;
    let mut command_options = None;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = value.trim().trim_end_matches(',').trim();
        if key.eq_ignore_ascii_case("name") {
            if name.is_some() {
                return None;
            }
            name = Some(parse_maybe_static_query_text(static_source, value)?);
        } else if key.eq_ignore_ascii_case("spawn") {
            if command.is_some() || command_options.is_some() {
                return None;
            }
            command =
                spawn_command_table_from_query_with_static_source(static_source, value, false);
            if command.is_none() {
                command_options = spawn_command_table_options_from_query_with_static_source(
                    static_source,
                    value,
                    false,
                );
            }
            if command.is_none() && command_options.is_none() {
                return None;
            }
        } else {
            return None;
        }
    }

    Some(WindowSwitchToWorkspaceOptions {
        name,
        command,
        command_options,
    })
}

fn switch_workspace_spawn_from_query(
    query: &str,
) -> (
    Option<WindowSpawnCommandQuery>,
    Option<WindowSpawnCommandQueryOptions>,
) {
    let query = format!("spawn {query}");
    let command = spawn_command_query_from_prefix(&query, "spawn ");
    let command_options = command
        .is_none()
        .then(|| spawn_command_options_from_prefix(&query, "spawn "))
        .flatten()
        .filter(|options| options.window_position.is_none());
    (command, command_options)
}

fn switch_workspace_query_name(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let name = strip_query_prefix_from_any(name, &["name=", "name "]).unwrap_or(name);

    let quoted = name.starts_with('"') || name.starts_with('\'');
    if quoted {
        let words = command_palette_query_words(name)?;
        if words.len() == 1 && !words[0].is_empty() {
            return Some(words[0].clone());
        }
    }

    Some(name.to_owned())
}

fn split_unquoted_query_marker<'a>(query: &'a str, marker: &str) -> Option<(&'a str, &'a str)> {
    let mut quote = None;
    let mut escaped = false;

    for (index, character) in query.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match quote {
            Some(_) if character == '\\' => escaped = true,
            Some(active_quote) if character == active_quote => quote = None,
            None if starts_with_ascii_case_insensitive(&query[index..], marker) => {
                return Some((&query[..index], &query[index + marker.len()..]));
            }
            None if character == '"' || character == '\'' => quote = Some(character),
            Some(_) | None => {}
        }
    }

    None
}

fn starts_with_ascii_case_insensitive(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
}

fn activate_window_command_from_query(query: &str) -> Option<WindowCommand> {
    activate_window_command_from_query_with_static_source(None, query)
}

fn activate_window_command_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCommand> {
    activate_window_relative_no_wrap_from_query_with_static_source(static_source, query)
        .map(WindowCommand::ActivateWindowRelativeNoWrap)
        .or_else(|| {
            activate_window_relative_from_query_with_static_source(static_source, query)
                .map(WindowCommand::ActivateWindowRelative)
        })
        .or_else(|| {
            activate_window_from_query_with_static_source(static_source, query)
                .map(WindowCommand::ActivateWindow)
        })
}

fn activate_window_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<usize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(index) = strip_lua_function_call_from_query(query, "activatewindow") {
        return parse_maybe_static_query_usize(static_source, index);
    }

    let index = strip_query_prefix_from_any(
        query,
        &[
            "activate window index=",
            "activate window index ",
            "activate window=",
            "activate window ",
            "activatewindow=",
            "activatewindow ",
        ],
    )
    .and_then(parse_single_query_value)?;
    strip_query_prefix_from_any(index, &["index=", "index "])
        .or(Some(index))
        .and_then(|index| parse_maybe_static_query_usize(static_source, index))
}

fn activate_window_relative_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(offset) = strip_lua_function_call_from_query(query, "activatewindowrelative") {
        return parse_maybe_static_query_isize(static_source, offset);
    }

    let offset = strip_query_prefix_from_any(
        query,
        &[
            "activate window relative=",
            "activate window relative ",
            "activatewindowrelative=",
            "activatewindowrelative ",
        ],
    )
    .and_then(parse_single_query_value)?;
    strip_query_prefix_from_any(offset, &["offset=", "offset ", "amount=", "amount "])
        .or(Some(offset))
        .and_then(|offset| parse_maybe_static_query_isize(static_source, offset))
}

fn activate_window_relative_no_wrap_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(offset) = strip_lua_function_call_from_query(query, "activatewindowrelativenowrap")
    {
        return parse_maybe_static_query_isize(static_source, offset);
    }

    let offset = strip_query_prefix_from_any(
        query,
        &[
            "activate window relative no wrap=",
            "activate window relative no wrap ",
            "activate window relative no-wrap=",
            "activate window relative no-wrap ",
            "activate window relative nowrap=",
            "activate window relative nowrap ",
            "activatewindowrelativenowrap=",
            "activatewindowrelativenowrap ",
            "activatewindowrelativeno-wrap=",
            "activatewindowrelativeno-wrap ",
            "activatewindowrelativeno wrap=",
            "activatewindowrelativeno wrap ",
        ],
    )
    .and_then(parse_single_query_value)?;
    strip_query_prefix_from_any(offset, &["offset=", "offset ", "amount=", "amount "])
        .or(Some(offset))
        .and_then(|offset| parse_maybe_static_query_isize(static_source, offset))
}

fn activate_tab_from_query(query: &str) -> Option<isize> {
    activate_tab_from_query_with_static_source(None, query)
}

fn activate_tab_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(index) = strip_lua_function_call_from_query(query, "activatetab") {
        return parse_maybe_static_query_isize(static_source, index);
    }

    let index = strip_query_prefix_from_any(
        query,
        &[
            "activate tab index=",
            "activate tab index ",
            "activate tab=",
            "activate tab ",
            "activatetab=",
            "activatetab ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(index, &["index=", "index "])
        .or(Some(index))
        .and_then(|index| parse_maybe_static_query_isize(static_source, index))
}

fn activate_tab_relative_from_query(query: &str) -> Option<isize> {
    activate_tab_relative_from_query_with_static_source(None, query)
}

fn activate_tab_relative_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(offset) = strip_lua_function_call_from_query(query, "activatetabrelative") {
        return parse_maybe_static_query_isize(static_source, offset);
    }

    let offset = strip_query_prefix_from_any(
        query,
        &[
            "activate tab relative=",
            "activate tab relative ",
            "activatetabrelative=",
            "activatetabrelative ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(offset, &["offset=", "offset ", "amount=", "amount "])
        .or(Some(offset))
        .and_then(|offset| parse_maybe_static_query_isize(static_source, offset))
}

fn activate_tab_relative_no_wrap_from_query(query: &str) -> Option<isize> {
    activate_tab_relative_no_wrap_from_query_with_static_source(None, query)
}

fn activate_tab_relative_no_wrap_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(offset) = strip_lua_function_call_from_query(query, "activatetabrelativenowrap") {
        return parse_maybe_static_query_isize(static_source, offset);
    }

    let offset = strip_query_prefix_from_any(
        query,
        &[
            "activate tab relative no wrap=",
            "activate tab relative no wrap ",
            "activate tab relative no-wrap=",
            "activate tab relative no-wrap ",
            "activate tab relative nowrap=",
            "activate tab relative nowrap ",
            "activatetabrelativenowrap=",
            "activatetabrelativenowrap ",
            "activatetabrelativeno-wrap=",
            "activatetabrelativeno-wrap ",
            "activatetabrelativeno wrap=",
            "activatetabrelativeno wrap ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(offset, &["offset=", "offset ", "amount=", "amount "])
        .or(Some(offset))
        .and_then(|offset| parse_maybe_static_query_isize(static_source, offset))
}

fn move_tab_from_query(query: &str) -> Option<usize> {
    move_tab_from_query_with_static_source(None, query)
}

fn move_tab_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<usize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(index) = strip_lua_function_call_from_query(query, "movetab") {
        return parse_maybe_static_query_usize(static_source, index);
    }

    let index = strip_query_prefix_from_any(
        query,
        &[
            "move tab to=",
            "move tab to ",
            "move tab=",
            "move tab ",
            "movetab=",
            "movetab ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(index, &["index=", "index "])
        .or(Some(index))
        .and_then(|index| parse_maybe_static_query_usize(static_source, index))
}

fn move_tab_to_window_from_query(query: &str) -> Option<rterm_core::WindowId> {
    move_tab_to_window_from_query_with_static_source(None, query)
}

fn move_tab_to_window_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<rterm_core::WindowId> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    let window_id = if let Some(window_id) = strip_lua_function_call_from_query(query, "movetabtowindow") {
        parse_maybe_static_query_usize(static_source, window_id)
    } else {
        let window_id = strip_query_prefix_from_any(
            query,
            &[
                "move tab to window=",
                "move tab to window ",
                "movetabtowindow=",
                "movetabtowindow ",
            ],
        )
        .and_then(parse_non_empty_query_text)?;
        let window_id = strip_query_prefix_from_any(window_id, &["window=", "window "])
            .or(Some(window_id))?;
        parse_maybe_static_query_usize(static_source, window_id)
    }?;
    u64::try_from(window_id).ok().map(rterm_core::WindowId::new)
}

fn move_tab_relative_from_query(query: &str) -> Option<isize> {
    move_tab_relative_from_query_with_static_source(None, query)
}

fn move_tab_relative_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(offset) = strip_lua_function_call_from_query(query, "movetabrelative") {
        return parse_maybe_static_query_isize(static_source, offset);
    }

    let offset = strip_query_prefix_from_any(
        query,
        &[
            "move tab relative=",
            "move tab relative ",
            "movetabrelative=",
            "movetabrelative ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(offset, &["offset=", "offset ", "amount=", "amount "])
        .or(Some(offset))
        .and_then(|offset| parse_maybe_static_query_isize(static_source, offset))
}

fn activate_pane_by_index_from_query(query: &str) -> Option<usize> {
    activate_pane_by_index_from_query_with_static_source(None, query)
}

fn activate_pane_by_index_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<usize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(index) = strip_lua_function_call_from_query(query, "activatepanebyindex") {
        return parse_maybe_static_query_usize(static_source, index);
    }

    let index = strip_query_prefix_from_any(
        query,
        &[
            "activate pane by index=",
            "activate pane by index ",
            "activate pane=",
            "activate pane ",
            "activatepanebyindex=",
            "activatepanebyindex ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(index, &["index=", "index "])
        .or(Some(index))
        .and_then(|index| parse_maybe_static_query_usize(static_source, index))
}

fn activate_pane_direction_from_query(query: &str) -> Option<PaneDirection> {
    activate_pane_direction_from_query_with_static_source(None, query)
}

fn activate_pane_direction_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<PaneDirection> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(direction) = strip_lua_function_call_from_query(query, "activatepanedirection")
        .and_then(|direction| parse_maybe_static_query_text(static_source, direction))
        .and_then(|direction| pane_direction_from_query(&direction))
    {
        return Some(direction);
    }

    if let Some(direction) = strip_query_prefix_from_any(query, &["activatepanedirection "])
        .and_then(|direction| parse_maybe_static_query_text(static_source, direction))
        .and_then(|direction| pane_direction_from_query(&direction))
    {
        return Some(direction);
    }

    let direction = strip_query_prefix_from_any(
        query,
        &[
            "activate pane direction=",
            "activate pane direction ",
            "activatepanedirection=",
            "activatepanedirection ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    let direction =
        strip_query_prefix_from_any(direction, &["direction=", "direction "]).unwrap_or(direction);
    let direction = parse_maybe_static_query_text(static_source, direction)?;
    pane_direction_from_query(&direction)
}

fn pane_direction_from_query(direction: &str) -> Option<PaneDirection> {
    match direction.to_ascii_lowercase().as_str() {
        "left" => Some(PaneDirection::Left),
        "right" => Some(PaneDirection::Right),
        "up" => Some(PaneDirection::Up),
        "down" => Some(PaneDirection::Down),
        "next" => Some(PaneDirection::Next),
        "prev" | "previous" => Some(PaneDirection::Previous),
        _ => None,
    }
}

fn adjust_pane_size_from_query(query: &str) -> Option<(ResizeDirection, u16)> {
    adjust_pane_size_from_query_with_static_source(None, query)
}

fn adjust_pane_size_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<(ResizeDirection, u16)> {
    if let Some(adjustment) =
        adjust_pane_size_table_from_query_with_static_source(static_source, query)
    {
        return Some(adjustment);
    }

    let rest = strip_query_prefix_from_any(
        query,
        &[
            "adjust pane size=",
            "adjust pane size ",
            "adjustpanesize=",
            "adjustpanesize ",
        ],
    )?;
    if let Some(adjustment) =
        adjust_pane_size_fields_from_query_with_static_source(static_source, rest)
    {
        return Some(adjustment);
    }
    let (direction, amount) = rest.split_once(char::is_whitespace)?;
    let direction = parse_maybe_static_query_text(static_source, direction)?;
    let direction = resize_direction_from_query(&direction)?;
    let amount = parse_maybe_static_query_u16(static_source, amount)?;
    Some((direction, amount))
}

fn adjust_pane_size_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<(ResizeDirection, u16)> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };
    let rest = strip_lua_function_call_from_query(query, "adjustpanesize")
        .or_else(|| strip_query_table_assignment_from_prefix(query, "adjustpanesize="))?;
    let rest = rest.trim();
    let resolved_rest;
    let rest = if rest.starts_with('{') {
        rest
    } else {
        let static_source = static_source?;
        resolved_rest = lua_table_insert_value_table_string_from_query(
            static_source.source,
            rest,
            static_source.max_start,
        )?;
        resolved_rest.as_str()
    };
    let table = rest.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let fields = split_lua_table_top_level_fields(table)?
        .into_iter()
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    if fields.len() != 2 {
        return None;
    }
    let direction = parse_maybe_static_query_text(static_source, fields[0])?;
    let amount = parse_maybe_static_query_u16(static_source, fields[1])?;
    Some((resize_direction_from_query(&direction)?, amount))
}

fn adjust_pane_size_fields_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    rest: &str,
) -> Option<(ResizeDirection, u16)> {
    let tokens = command_palette_query_words(rest)?;
    let mut direction = None;
    let mut amount = None;
    let mut parsed_structured_field = false;
    let mut index = 0;

    while index < tokens.len() {
        let token = tokens[index].as_str();
        if let Some(value) = query_assignment_value_from_token(token, &["direction"]) {
            if direction.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            direction = Some(resize_direction_from_query(&value)?);
            parsed_structured_field = true;
            index += 1;
            continue;
        }

        if let Some(value) = query_assignment_value_from_token(token, &["amount"]) {
            if amount.is_some() {
                return None;
            }
            amount = Some(parse_maybe_static_query_u16(static_source, value)?);
            parsed_structured_field = true;
            index += 1;
            continue;
        }

        let token_key = token.to_ascii_lowercase();
        match token_key.as_str() {
            "direction" => {
                if direction.is_some() {
                    return None;
                }
                let value =
                    parse_maybe_static_query_text(static_source, tokens.get(index + 1)?.as_str())?;
                direction = Some(resize_direction_from_query(&value)?);
                parsed_structured_field = true;
                index += 2;
            }
            "amount" => {
                if amount.is_some() {
                    return None;
                }
                amount = Some(parse_maybe_static_query_u16(
                    static_source,
                    tokens.get(index + 1)?.as_str(),
                )?);
                parsed_structured_field = true;
                index += 2;
            }
            _ => return None,
        }
    }

    if parsed_structured_field {
        Some((direction?, amount?))
    } else {
        None
    }
}

fn resize_direction_from_query(direction: &str) -> Option<ResizeDirection> {
    match direction.to_ascii_lowercase().as_str() {
        "left" => Some(ResizeDirection::Left),
        "right" => Some(ResizeDirection::Right),
        "up" => Some(ResizeDirection::Up),
        "down" => Some(ResizeDirection::Down),
        _ => None,
    }
}

fn scroll_by_page_from_query(query: &str) -> Option<WindowScrollByPageAmount> {
    scroll_by_page_from_query_with_static_source(None, query)
}

fn scroll_by_page_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowScrollByPageAmount> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(amount) = strip_lua_function_call_from_query(query, "scrollbypage") {
        return parse_maybe_static_query_f64(static_source, amount)
            .and_then(scroll_by_page_amount_from_f64);
    }

    let amount = strip_query_prefix_from_any(
        query,
        &[
            "scroll by page=",
            "scroll by page ",
            "scrollbypage=",
            "scrollbypage ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    let amount = strip_query_prefix_from_any(amount, &["amount=", "amount ", "offset=", "offset "])
        .unwrap_or(amount);
    parse_maybe_static_query_f64(static_source, amount).and_then(scroll_by_page_amount_from_f64)
}

fn scroll_by_line_from_query(query: &str) -> Option<isize> {
    scroll_by_line_from_query_with_static_source(None, query)
}

fn scroll_by_line_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(amount) = strip_lua_function_call_from_query(query, "scrollbyline") {
        return parse_maybe_static_query_isize(static_source, amount);
    }

    let amount = strip_query_prefix_from_any(
        query,
        &[
            "scroll by line=",
            "scroll by line ",
            "scrollbyline=",
            "scrollbyline ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(amount, &["amount=", "amount ", "offset=", "offset "])
        .or(Some(amount))
        .and_then(|amount| parse_maybe_static_query_isize(static_source, amount))
}

fn scroll_to_prompt_from_query(query: &str) -> Option<isize> {
    scroll_to_prompt_from_query_with_static_source(None, query)
}

fn scroll_to_prompt_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<isize> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(amount) = strip_lua_function_call_from_query(query, "scrolltoprompt") {
        return parse_maybe_static_query_isize(static_source, amount);
    }

    let amount = strip_query_prefix_from_any(
        query,
        &[
            "scroll to prompt=",
            "scroll to prompt ",
            "scrolltoprompt=",
            "scrolltoprompt ",
        ],
    )
    .and_then(parse_non_empty_query_text)?;
    strip_query_prefix_from_any(amount, &["amount=", "amount ", "offset=", "offset "])
        .or(Some(amount))
        .and_then(|amount| parse_maybe_static_query_isize(static_source, amount))
}

#[allow(clippy::cast_possible_truncation)]
fn scroll_by_page_amount_from_f64(amount: f64) -> Option<WindowScrollByPageAmount> {
    amount
        .is_finite()
        .then_some(WindowScrollByPageAmount::from_per_mille(
            (amount * f64::from(WINDOW_SCROLL_PAGE_AMOUNT_SCALE)).round() as i32,
        ))
}

fn set_pane_zoom_state_from_query(query: &str) -> Option<bool> {
    set_pane_zoom_state_from_query_with_static_source(None, query)
}

fn set_pane_zoom_state_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<bool> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(value) = strip_lua_function_call_from_query(query, "setpanezoomstate") {
        return parse_maybe_static_query_bool(static_source, value.trim());
    }

    let value = strip_query_prefix_from_any(query, &["set pane zoom state ", "setpanezoomstate "])
        .or_else(|| {
            ["set pane zoom state ", "setpanezoomstate "]
                .iter()
                .find_map(|prefix| {
                    let equals_prefix = prefix.trim_end().to_owned() + "=";
                    strip_query_prefix_from_any(query, &[equals_prefix.as_str()])
                })
        })?;
    let value = strip_query_prefix_from_any(value, &["zoomed=", "zoomed ", "value=", "value "])
        .unwrap_or(value);
    parse_maybe_static_query_bool(static_source, value)
}

fn strip_query_prefix_from_any<'a>(query: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let query = query.trim();
    prefixes.iter().find_map(|prefix| {
        let candidate = query.get(..prefix.len())?;
        let rest = query.get(prefix.len()..)?;
        candidate
            .eq_ignore_ascii_case(prefix)
            .then_some(rest.trim())
    })
}

fn bool_from_query(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn rotate_panes_from_query(query: &str) -> Option<PaneRotationDirection> {
    rotate_panes_from_query_with_static_source(None, query)
}

fn rotate_panes_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<PaneRotationDirection> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(direction) = strip_lua_function_call_from_query(query, "rotatepanes")
        .and_then(|direction| parse_maybe_static_query_text(static_source, direction))
        .and_then(|direction| pane_rotation_direction_from_query(&direction))
    {
        return Some(direction);
    }

    let direction = strip_query_prefix_from_any(
        query,
        &[
            "rotate panes=",
            "rotate panes ",
            "rotatepanes=",
            "rotatepanes ",
        ],
    )?;
    let direction =
        strip_query_prefix_from_any(direction, &["direction=", "direction "]).unwrap_or(direction);
    let direction = parse_maybe_static_query_text(static_source, direction)?;
    pane_rotation_direction_from_query(&direction)
}

fn pane_rotation_direction_from_query(direction: &str) -> Option<PaneRotationDirection> {
    let normalized = direction
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "clockwise" => Some(PaneRotationDirection::Clockwise),
        "counterclockwise" => Some(PaneRotationDirection::CounterClockwise),
        _ => None,
    }
}

fn set_window_level_from_query(query: &str) -> Option<NativeWindowLevel> {
    set_window_level_from_query_with_static_source(None, query)
}

fn set_window_level_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<NativeWindowLevel> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(level) = strip_lua_function_call_from_query(query, "setwindowlevel") {
        let level = parse_maybe_static_query_text(static_source, level)?;
        return native_window_level_from_query(&level);
    }

    let level = strip_query_prefix_from_any(
        query,
        &[
            "set window level=",
            "set window level ",
            "setwindowlevel=",
            "setwindowlevel ",
        ],
    )?;
    let level = strip_query_prefix_from_any(level, &["level=", "level "]).unwrap_or(level);
    let level = parse_maybe_static_query_text(static_source, level)?;
    native_window_level_from_query(&level)
}

fn mouse_selection_command_from_query(query: &str) -> Option<WindowCommand> {
    select_text_at_mouse_cursor_mode_from_query(query)
        .map(WindowCommand::SelectTextAtMouseCursor)
        .or_else(|| {
            extend_selection_to_mouse_cursor_mode_from_query(query)
                .map(WindowCommand::ExtendSelectionToMouseCursor)
        })
}

fn select_text_at_mouse_cursor_mode_from_query(query: &str) -> Option<WindowMouseSelectionMode> {
    select_text_at_mouse_cursor_mode_from_query_with_static_source(None, query)
}

fn select_text_at_mouse_cursor_mode_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowMouseSelectionMode> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(mode) = strip_lua_function_call_from_query(query, "selecttextatmousecursor") {
        let mode = parse_maybe_static_query_text(static_source, mode)?;
        return mouse_selection_mode_from_query(&mode);
    }

    let mode = strip_query_prefix_from_any(
        query,
        &[
            "select text at mouse cursor=",
            "select text at mouse cursor ",
            "selecttextatmousecursor=",
            "selecttextatmousecursor ",
        ],
    )?;
    let mode = strip_query_prefix_from_any(mode, &["mode=", "mode "]).unwrap_or(mode);
    let mode = parse_maybe_static_query_text(static_source, mode)?;
    mouse_selection_mode_from_query(&mode)
}

fn extend_selection_to_mouse_cursor_mode_from_query(
    query: &str,
) -> Option<WindowMouseSelectionMode> {
    extend_selection_to_mouse_cursor_mode_from_query_with_static_source(None, query)
}

fn extend_selection_to_mouse_cursor_mode_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowMouseSelectionMode> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(mode) = strip_lua_function_call_from_query(query, "extendselectiontomousecursor") {
        let mode = parse_maybe_static_query_text(static_source, mode)?;
        return mouse_selection_mode_from_query(&mode);
    }

    let mode = strip_query_prefix_from_any(
        query,
        &[
            "extend selection to mouse cursor=",
            "extend selection to mouse cursor ",
            "extendselectiontomousecursor=",
            "extendselectiontomousecursor ",
        ],
    )?;
    let mode = strip_query_prefix_from_any(mode, &["mode=", "mode "]).unwrap_or(mode);
    let mode = parse_maybe_static_query_text(static_source, mode)?;
    mouse_selection_mode_from_query(&mode)
}

fn mouse_selection_mode_from_query(mode: &str) -> Option<WindowMouseSelectionMode> {
    let normalized = mode
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "cell" => Some(WindowMouseSelectionMode::Cell),
        "word" => Some(WindowMouseSelectionMode::Word),
        "line" => Some(WindowMouseSelectionMode::Line),
        "block" => Some(WindowMouseSelectionMode::Block),
        "semanticzone" => Some(WindowMouseSelectionMode::SemanticZone),
        _ => None,
    }
}

fn char_select_options_from_query(query: &str) -> Option<WindowCharSelectOptions> {
    char_select_options_from_query_with_static_source(None, query)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn char_select_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCharSelectOptions> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    if let Some(rest) = strip_lua_function_call_from_query(query, "charselect") {
        let rest = rest.trim();
        if rest.starts_with('{') {
            return char_select_lua_table_from_query_with_static_source(static_source, rest);
        }
        if static_source.is_some()
            && let Some(options) =
                char_select_lua_table_from_query_with_static_source(static_source, rest)
        {
            return Some(options);
        }
    }

    if let Some(rest) = strip_query_table_assignment_from_prefix(query, "charselect=")
        && rest.trim_start().starts_with('{')
    {
        return char_select_lua_table_from_query_with_static_source(static_source, rest);
    }

    if static_source.is_some() {
        return char_select_options_from_query_with_static_source(None, query);
    }

    let rest = strip_query_prefix_from_any(
        query,
        &["char select=", "char select ", "charselect=", "charselect "],
    )?;
    if rest.is_empty() {
        return None;
    }

    let token_values = command_palette_query_words(rest)?;
    let tokens = token_values.iter().map(String::as_str).collect::<Vec<_>>();
    let mut index = 0;
    let mut parsed = false;
    let mut parsed_copy_on_select = false;
    let mut parsed_copy_to = false;
    let mut parsed_group = false;
    let mut options = WindowCharSelectOptions::default();

    while index < tokens.len() {
        if let Some(value) =
            query_assignment_value_from_token(tokens[index], &["copy_on_select", "copy-on-select"])
        {
            if parsed_copy_on_select {
                return None;
            }
            options.copy_on_select = bool_from_query(value)?;
            index += 1;
            parsed = true;
            parsed_copy_on_select = true;
            continue;
        }

        if let Some(value) =
            query_text_assignment_value_from_token(tokens[index], &["copy_to", "copy-to"])
        {
            if parsed_copy_to {
                return None;
            }
            options.copy_to = copy_destination_from_query(value)?;
            index += 1;
            parsed = true;
            parsed_copy_to = true;
            continue;
        }

        if let Some(value) = query_text_assignment_value_from_token(tokens[index], &["group"]) {
            if parsed_group {
                return None;
            }
            options.group = Some(value.to_owned());
            index += 1;
            parsed = true;
            parsed_group = true;
            continue;
        }

        if char_select_copy_on_select_field_at(&tokens, index) {
            if parsed_copy_on_select {
                return None;
            }
            let field_len = char_select_copy_on_select_field_len(&tokens, index)?;
            if let Some(value) =
                char_select_copy_on_select_field_value(&tokens, index + field_len - 1)
            {
                options.copy_on_select = value;
                index += field_len;
            } else {
                index += field_len;
                options.copy_on_select = bool_from_query(tokens.get(index)?)?;
                index += 1;
            }
            parsed = true;
            parsed_copy_on_select = true;
            continue;
        }

        if char_select_copy_to_field_at(&tokens, index) {
            if parsed_copy_to {
                return None;
            }
            let field_len = char_select_copy_to_field_len(&tokens, index)?;
            let inline_value = char_select_copy_to_field_value(&tokens, index + field_len - 1);
            index += field_len;
            let end = next_char_select_field_index(&tokens, index);
            let copy_to = if let Some(value) = inline_value {
                if end == index {
                    value.to_owned()
                } else {
                    format!("{value} {}", tokens[index..end].join(" "))
                }
            } else {
                if end == index {
                    return None;
                }
                tokens[index..end].join(" ")
            };
            options.copy_to = copy_destination_from_query(&copy_to)?;
            index = end;
            parsed = true;
            parsed_copy_to = true;
            continue;
        }

        if tokens[index].eq_ignore_ascii_case("group") {
            if parsed_group {
                return None;
            }
            index += 1;
            let end = next_char_select_field_index(&tokens, index);
            if end == index {
                return None;
            }
            options.group = Some(tokens[index..end].join(" "));
            index = end;
            parsed = true;
            parsed_group = true;
            continue;
        }

        return None;
    }

    parsed.then_some(options)
}

fn char_select_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowCharSelectOptions> {
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
    let mut options = WindowCharSelectOptions::default();
    let mut parsed = false;
    let mut parsed_copy_on_select = false;
    let mut parsed_copy_to = false;
    let mut parsed_group = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();

        match normalized_char_select_lua_field(&name).as_str() {
            "copyonselect" => {
                if parsed_copy_on_select {
                    return None;
                }
                options.copy_on_select = parse_maybe_static_query_bool(static_source, value)?;
                parsed_copy_on_select = true;
            }
            "copyto" => {
                if parsed_copy_to {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                options.copy_to = copy_destination_from_query(&value)?;
                parsed_copy_to = true;
            }
            "group" => {
                if parsed_group {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                options.group = Some(value);
                parsed_group = true;
            }
            _ => return None,
        }
        parsed = true;
    }

    parsed.then_some(options)
}

fn normalized_char_select_lua_field(field: &str) -> String {
    field
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn next_char_select_field_index(tokens: &[&str], mut index: usize) -> usize {
    while index < tokens.len() && !char_select_field_at(tokens, index) {
        index += 1;
    }
    index
}

fn char_select_field_at(tokens: &[&str], index: usize) -> bool {
    char_select_copy_on_select_field_at(tokens, index)
        || char_select_copy_to_field_at(tokens, index)
        || tokens
            .get(index)
            .is_some_and(|token| token.eq_ignore_ascii_case("group"))
}

fn char_select_copy_on_select_field_at(tokens: &[&str], index: usize) -> bool {
    tokens.get(index).is_some_and(|token| {
        token.eq_ignore_ascii_case("copy_on_select") || token.eq_ignore_ascii_case("copy-on-select")
    }) || matches!(
        tokens.get(index..index + 3),
        Some([copy, on, select])
            if copy.eq_ignore_ascii_case("copy")
                && on.eq_ignore_ascii_case("on")
                && (select.eq_ignore_ascii_case("select")
                    || starts_with_ascii_case_insensitive(select, "select="))
    )
}

fn char_select_copy_on_select_field_len(tokens: &[&str], index: usize) -> Option<usize> {
    char_select_copy_on_select_field_at(tokens, index).then(|| {
        if tokens
            .get(index)
            .is_some_and(|token| token.eq_ignore_ascii_case("copy"))
        {
            3
        } else {
            1
        }
    })
}

fn char_select_copy_on_select_field_value(tokens: &[&str], index: usize) -> Option<bool> {
    let token = tokens.get(index)?;
    query_assignment_value_from_token(token, &["copy_on_select", "copy-on-select", "select"])
        .and_then(bool_from_query)
}

fn char_select_copy_to_field_at(tokens: &[&str], index: usize) -> bool {
    tokens.get(index).is_some_and(|token| {
        token.eq_ignore_ascii_case("copy_to") || token.eq_ignore_ascii_case("copy-to")
    }) || matches!(
        tokens.get(index..index + 2),
        Some([copy, to])
            if copy.eq_ignore_ascii_case("copy")
                && (to.eq_ignore_ascii_case("to")
                    || starts_with_ascii_case_insensitive(to, "to="))
    )
}

fn char_select_copy_to_field_len(tokens: &[&str], index: usize) -> Option<usize> {
    char_select_copy_to_field_at(tokens, index).then(|| {
        if tokens
            .get(index)
            .is_some_and(|token| token.eq_ignore_ascii_case("copy"))
        {
            2
        } else {
            1
        }
    })
}

fn char_select_copy_to_field_value<'a>(tokens: &'a [&str], index: usize) -> Option<&'a str> {
    let token = tokens.get(index)?;
    query_assignment_value_from_token(token, &["copy_to", "copy-to", "to"])
}

fn native_window_level_from_query(level: &str) -> Option<NativeWindowLevel> {
    let normalized = level
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "alwaysonbottom" => Some(NativeWindowLevel::AlwaysOnBottom),
        "normal" => Some(NativeWindowLevel::Normal),
        "alwaysontop" => Some(NativeWindowLevel::AlwaysOnTop),
        _ => None,
    }
}

fn complete_selection_command_from_query(query: &str) -> Option<WindowCommand> {
    complete_selection_or_open_link_destination_from_query(query)
        .map(WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo)
        .or_else(|| {
            complete_selection_destination_from_query(query).map(WindowCommand::CompleteSelectionTo)
        })
}

fn complete_selection_destination_from_query(query: &str) -> Option<WindowCopyDestination> {
    complete_selection_destination_from_query_with_static_source(None, query)
}

fn complete_selection_destination_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCopyDestination> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(destination) = strip_lua_function_call_from_query(query, "completeselection") {
        return copy_destination_from_query_with_static_source(static_source, destination);
    }

    let destination = strip_query_prefix_from_any(
        query,
        &[
            "complete selection to=",
            "complete selection to ",
            "completeselection=",
            "completeselection ",
            "completeselectionto=",
            "completeselectionto ",
        ],
    )?;
    let destination = strip_query_prefix_from_any(destination, &["destination=", "destination "])
        .unwrap_or(destination);
    copy_destination_from_query_with_static_source(static_source, destination)
}

fn complete_selection_or_open_link_destination_from_query(
    query: &str,
) -> Option<WindowCopyDestination> {
    complete_selection_or_open_link_destination_from_query_with_static_source(None, query)
}

fn complete_selection_or_open_link_destination_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowCopyDestination> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(destination) =
        strip_lua_function_call_from_query(query, "completeselectionoropenlinkatmousecursor")
    {
        return copy_destination_from_query_with_static_source(static_source, destination);
    }

    let destination = strip_query_prefix_from_any(
        query,
        &[
            "complete selection open link to=",
            "complete selection open link to ",
            "complete selection or open link at mouse cursor to=",
            "complete selection or open link at mouse cursor to ",
            "completeselectionoropenlinkatmousecursor=",
            "completeselectionoropenlinkatmousecursor ",
            "completeselectionoropenlinkatmousecursorto=",
            "completeselectionoropenlinkatmousecursorto ",
        ],
    )?;
    let destination = strip_query_prefix_from_any(destination, &["destination=", "destination "])
        .unwrap_or(destination);
    copy_destination_from_query_with_static_source(static_source, destination)
}

fn copy_text_to_from_query(query: &str) -> Option<(String, WindowCopyDestination)> {
    copy_text_to_from_query_with_static_source(None, query)
}

fn copy_text_to_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<(String, WindowCopyDestination)> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(value) = strip_lua_function_call_from_query(query, "copytextto") {
        let value = value.trim();
        if value.starts_with('{') {
            return copy_text_to_lua_table_from_query_with_static_source(static_source, value);
        }
        if static_source.is_some()
            && let Some(command) =
                copy_text_to_lua_table_from_query_with_static_source(static_source, value)
        {
            return Some(command);
        }
    }

    if let Some(value) = strip_query_table_assignment_from_prefix(query, "copytextto=")
        && value.trim_start().starts_with('{')
    {
        return copy_text_to_lua_table_from_query_with_static_source(static_source, value);
    }

    None
}

fn copy_text_to_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<(String, WindowCopyDestination)> {
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
    let mut text = None;
    let mut destination = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        match normalized_action_name_query(&name).as_str() {
            "text" => {
                if text.is_some() {
                    return None;
                }
                text = Some(parse_maybe_static_query_text(static_source, value.trim())?);
            }
            "destination" => {
                if destination.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value.trim())?;
                destination = Some(copy_destination_from_query(&value)?);
            }
            _ => return None,
        }
    }

    Some((text?, destination?))
}

fn copy_destination_from_query(destination: &str) -> Option<WindowCopyDestination> {
    copy_destination_from_query_with_static_source(None, destination)
}

fn copy_destination_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    destination: &str,
) -> Option<WindowCopyDestination> {
    let destination = strip_query_prefix_from_any(destination, &["destination=", "destination "])
        .unwrap_or(destination);
    let destination = parse_maybe_static_query_text(static_source, destination)?;
    let normalized = destination
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "clipboard" => Some(WindowCopyDestination::Clipboard),
        "primary" | "primaryselection" => Some(WindowCopyDestination::PrimarySelection),
        "clipboardprimary"
        | "clipboardandprimary"
        | "clipboardprimaryselection"
        | "clipboardandprimaryselection" => {
            Some(WindowCopyDestination::ClipboardAndPrimarySelection)
        }
        _ => None,
    }
}

fn parse_single_query_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() || value.contains(char::is_whitespace) {
        None
    } else {
        Some(value)
    }
}

fn parse_non_empty_query_text(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn parse_maybe_static_query_text(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_color_string_from_query(static_source, value)
    {
        return (!value.is_empty()).then_some(value);
    }
    if let Some(value) = lua_static_string_value_from_expression(static_source, None, value) {
        return (!value.is_empty()).then_some(value);
    }

    parse_maybe_quoted_query_text(value)
}

fn parse_maybe_static_query_text_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    if let Some(value) = static_source
        .and_then(|source| lua_static_color_string_from_query(source, value))
        .or_else(|| {
            outer_static_source.and_then(|source| lua_static_color_string_from_query(source, value))
        })
    {
        return (!value.is_empty()).then_some(value);
    }
    if let Some(value) =
        lua_static_string_value_from_expression(static_source, outer_static_source, value)
    {
        return (!value.is_empty()).then_some(value);
    }

    parse_maybe_quoted_query_text(value)
}

fn parse_maybe_static_query_bool(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<bool> {
    if let Some(value) =
        static_source.and_then(|source| lua_static_color_bool_from_query(source, value))
    {
        return Some(value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_bool_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return bool_from_query(value);
    }

    let value = parse_maybe_static_query_text(static_source, value)?;
    bool_from_query(&value)
}

fn parse_maybe_static_query_bool_with_static_sources(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<bool> {
    if let Some(value) = static_source
        .and_then(|source| lua_static_color_bool_from_query(source, value))
        .or_else(|| {
            outer_static_source.and_then(|source| lua_static_color_bool_from_query(source, value))
        })
    {
        return Some(value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_bool_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return bool_from_query(value);
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_bool_assignment_value_before_offset_from_query(
            outer_static_source.source,
            value,
            outer_static_source.max_start,
        )
    {
        return bool_from_query(value);
    }

    let value = parse_maybe_static_query_text_with_static_sources(
        static_source,
        outer_static_source,
        value,
    )?;
    bool_from_query(&value)
}

fn parse_maybe_static_query_u16(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<u16> {
    let value = if let Some(static_source) = static_source {
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
    };
    parse_single_query_value(&value)?.parse().ok()
}

fn parse_maybe_static_query_usize(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<usize> {
    let value = if let Some(static_source) = static_source {
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
    };
    parse_single_query_value(&value)?.parse().ok()
}

fn parse_maybe_static_query_isize(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<isize> {
    let value = if let Some(static_source) = static_source {
        lua_static_number_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
            lua_signed_number_literal_from_query,
        )
        .map(str::to_owned)
        .or_else(|| parse_maybe_quoted_query_text(value))?
    } else {
        parse_maybe_quoted_query_text(value)?
    };
    parse_single_query_value(&value)?.parse().ok()
}

fn parse_maybe_static_query_f64(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<f64> {
    if let Some(value) =
        static_source.and_then(|source| lua_static_color_number_from_query(source, value))
    {
        return Some(value);
    }
    let value = if let Some(static_source) = static_source {
        lua_static_number_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
            lua_signed_number_literal_from_query,
        )
        .map(str::to_owned)
        .or_else(|| parse_maybe_quoted_query_text(value))?
    } else {
        parse_maybe_quoted_query_text(value)?
    };
    parse_single_query_value(&value)?.parse().ok()
}

fn lua_static_string_assignment_value_before_offset_from_query<'a>(
    source: &'a str,
    query: &str,
    max_start: usize,
) -> Option<&'a str> {
    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)
}

fn parse_maybe_quoted_query_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with('"') || value.starts_with('\'') {
        return parse_lua_quoted_query_text(value).filter(|value| !value.is_empty());
    }
    if value.starts_with('[') {
        return parse_lua_long_bracket_query_text(value).filter(|value| !value.is_empty());
    }
    parse_non_empty_query_text(value).map(str::to_owned)
}

fn parse_lua_quoted_query_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with('[') {
        return parse_lua_long_bracket_query_text(value);
    }
    let mut chars = value.char_indices();
    let (_, quote) = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let mut parsed = String::new();
    while let Some((index, character)) = chars.next() {
        if character == quote {
            let rest = &value[index + character.len_utf8()..];
            return rest.trim().is_empty().then_some(parsed);
        }
        if character != '\\' {
            parsed.push(character);
            continue;
        }

        let (_, escaped) = chars.next()?;
        match escaped {
            'a' => parsed.push('\u{7}'),
            'b' => parsed.push('\u{8}'),
            'f' => parsed.push('\u{c}'),
            'n' => parsed.push('\n'),
            'r' => parsed.push('\r'),
            't' => parsed.push('\t'),
            'v' => parsed.push('\u{b}'),
            '\\' => parsed.push('\\'),
            '"' => parsed.push('"'),
            '\'' => parsed.push('\''),
            digit if digit.is_ascii_digit() => {
                let mut digits = String::from(digit);
                for _ in 0..2 {
                    let mut next_chars = chars.clone();
                    let Some((_, next)) = next_chars.next() else {
                        break;
                    };
                    if !next.is_ascii_digit() {
                        break;
                    }
                    chars.next();
                    digits.push(next);
                }
                let byte = digits.parse::<u8>().ok()?;
                parsed.push(char::from(byte));
            }
            'x' => {
                let (_, high) = chars.next()?;
                let (_, low) = chars.next()?;
                let hex = [high, low].into_iter().collect::<String>();
                let byte = u8::from_str_radix(&hex, 16).ok()?;
                parsed.push(char::from(byte));
            }
            'u' => {
                let (_, open) = chars.next()?;
                if open != '{' {
                    return None;
                }
                let mut hex = String::new();
                loop {
                    let (_, next) = chars.next()?;
                    if next == '}' {
                        break;
                    }
                    if !next.is_ascii_hexdigit() {
                        return None;
                    }
                    hex.push(next);
                }
                if hex.is_empty() {
                    return None;
                }
                let scalar = u32::from_str_radix(&hex, 16).ok()?;
                parsed.push(char::from_u32(scalar)?);
            }
            'z' => loop {
                let mut next_chars = chars.clone();
                let Some((_, next)) = next_chars.next() else {
                    break;
                };
                if !next.is_whitespace() {
                    break;
                }
                chars.next();
            },
            _ => parsed.push(escaped),
        }
    }

    None
}

fn parse_lua_long_bracket_query_text(value: &str) -> Option<String> {
    let value = value.trim();
    let (content_start, closing) = parse_lua_long_bracket_delimiters(value)?;
    let content_and_rest = &value[content_start..];
    let close_index = content_and_rest.find(&closing)?;
    let mut content = &content_and_rest[..close_index];
    let rest = &content_and_rest[close_index + closing.len()..];
    if !rest.trim().is_empty() {
        return None;
    }

    if let Some(stripped) = content.strip_prefix("\r\n") {
        content = stripped;
    } else if let Some(stripped) = content.strip_prefix('\n') {
        content = stripped;
    }

    Some(content.to_owned())
}

fn parse_lua_long_bracket_delimiters(value: &str) -> Option<(usize, String)> {
    let mut chars = value.char_indices();
    let (_, first) = chars.next()?;
    if first != '[' {
        return None;
    }

    let mut level = 0usize;
    for (index, character) in chars {
        match character {
            '=' => level += 1,
            '[' => {
                return Some((
                    index + character.len_utf8(),
                    format!("]{}]", "=".repeat(level)),
                ));
            }
            _ => return None,
        }
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowSpawnCommandQuery {
    label: Option<String>,
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    environment: BTreeMap<String, String>,
    domain: Option<WindowSpawnTabDomain>,
    window_position: Option<WindowPosition>,
}

impl WindowSpawnCommandQuery {
    fn into_supported_pane_launch(self, default_domain: &str) -> Result<PaneLaunch, AppShellError> {
        if let Some(domain) = &self.domain
            && !domain.is_supported_local_domain(default_domain)
        {
            return Err(AppShellError::UnsupportedAction);
        }
        Ok(self.into_pane_launch())
    }

    fn into_pane_launch(self) -> PaneLaunch {
        let mut launch = PaneLaunch::local(self.program).with_args(self.args);
        if let Some(cwd) = self.cwd {
            launch = launch.with_cwd(cwd);
        }
        launch = launch.with_environment(self.environment);
        launch
    }

    fn launch_menu_label(&self) -> String {
        if let Some(label) = &self.label {
            return label.clone();
        }
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn spawn_command_in_new_tab_from_query(query: &str) -> Option<WindowSpawnCommandQuery> {
    spawn_command_in_new_tab_from_query_with_static_source(None, query)
}

fn spawn_command_in_new_tab_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSpawnCommandQuery> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };
    spawn_command_table_query_from_lua_function_with_static_source(
        static_source,
        query,
        "spawncommandinnewtab",
        false,
    )
    .or_else(|| {
        spawn_command_table_query_from_prefix_with_static_source(
            static_source,
            query,
            "new tab=",
            false,
        )
    })
    .or_else(|| {
        spawn_command_table_query_from_prefix_with_static_source(
            static_source,
            query,
            "spawncommandinnewtab=",
            false,
        )
    })
    .or_else(|| spawn_command_query_from_prefix(query, "new tab="))
    .or_else(|| spawn_command_query_from_prefix(query, "new tab "))
    .or_else(|| spawn_command_query_from_prefix(query, "spawncommandinnewtab="))
    .or_else(|| spawn_command_query_from_prefix(query, "spawncommandinnewtab "))
}

fn spawn_command_table_query_from_lua_function_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    name: &str,
    allow_position: bool,
) -> Option<WindowSpawnCommandQuery> {
    let command = strip_lua_function_call_from_query(query, name)?;
    spawn_command_table_from_query_with_static_source(static_source, command, allow_position)
}

fn spawn_command_table_query_from_prefix_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    prefix: &str,
    allow_position: bool,
) -> Option<WindowSpawnCommandQuery> {
    let command = strip_query_table_assignment_from_prefix(query, prefix)?;
    spawn_command_table_from_query_with_static_source(static_source, command, allow_position)
}

fn spawn_command_options_in_new_tab_from_query(
    query: &str,
) -> Option<WindowSpawnCommandQueryOptions> {
    spawn_command_options_in_new_tab_from_query_with_static_source(None, query)
}

fn spawn_command_options_in_new_tab_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSpawnCommandQueryOptions> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };
    spawn_command_table_options_from_lua_function_with_static_source(
        static_source,
        query,
        "spawncommandinnewtab",
        false,
    )
    .or_else(|| {
        spawn_command_table_options_from_prefix_with_static_source(
            static_source,
            query,
            "new tab=",
            false,
        )
    })
    .or_else(|| {
        spawn_command_table_options_from_prefix_with_static_source(
            static_source,
            query,
            "spawncommandinnewtab=",
            false,
        )
    })
    .or_else(|| spawn_command_options_from_prefix(query, "new tab="))
    .or_else(|| spawn_command_options_from_prefix(query, "new tab "))
    .or_else(|| spawn_command_options_from_prefix(query, "spawncommandinnewtab="))
    .or_else(|| spawn_command_options_from_prefix(query, "spawncommandinnewtab "))
    .filter(|options| options.window_position.is_none())
}

fn spawn_tab_domain_from_query(query: &str) -> Option<WindowSpawnTabDomain> {
    spawn_tab_domain_from_query_with_static_source(None, query)
}

fn spawn_tab_domain_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSpawnTabDomain> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(domain) = spawn_tab_domain_from_lua_table_with_static_source(static_source, query) {
        return Some(domain);
    }

    if let Some(domain) =
        spawn_tab_domain_from_lua_function_with_static_source(static_source, query)
    {
        return Some(domain);
    }

    let domain = strip_query_prefix_from_any(
        query,
        &["spawn tab=", "spawn tab ", "spawntab=", "spawntab "],
    )?;
    let domain = parse_maybe_static_query_text(static_source, domain)?;
    spawn_tab_domain_value_from_query(&domain)
}

fn spawn_tab_domain_from_lua_table_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSpawnTabDomain> {
    if let Some(domain) = strip_lua_function_call_from_query(query, "spawntab") {
        return spawn_tab_domain_lua_table_from_query_with_static_source(static_source, domain);
    }

    if let Some(domain) = strip_query_table_assignment_from_prefix(query, "spawntab=") {
        return spawn_tab_domain_lua_table_from_query_with_static_source(static_source, domain);
    }

    None
}

fn spawn_tab_domain_lua_table_from_query(value: &str) -> Option<WindowSpawnTabDomain> {
    spawn_tab_domain_lua_table_from_query_with_static_source(None, value)
}

fn spawn_tab_domain_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowSpawnTabDomain> {
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
    let mut domain = None;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if domain.is_some() {
            return None;
        }
        if key.eq_ignore_ascii_case("domainname") {
            let value = parse_maybe_static_query_text(static_source, value)?;
            domain = Some(WindowSpawnTabDomain::DomainName(value));
        } else if key.eq_ignore_ascii_case("domainid") {
            let value = parse_maybe_static_usize_query(static_source, value)?;
            domain = Some(WindowSpawnTabDomain::DomainId(value));
        } else {
            return None;
        }
    }
    domain
}

fn spawn_tab_domain_from_lua_function_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSpawnTabDomain> {
    let domain = strip_lua_function_call_from_query(query, "spawntab")?;
    let domain = parse_maybe_static_query_text(static_source, domain)?;
    spawn_tab_domain_value_from_query(&domain)
}

fn spawn_tab_domain_value_from_query(domain: &str) -> Option<WindowSpawnTabDomain> {
    let trimmed = domain.trim();
    if trimmed.starts_with('"') || trimmed.starts_with('\'') {
        let domain = parse_maybe_quoted_query_text(trimmed)?;
        return spawn_tab_domain_value_from_query(&domain);
    }

    if let Some(name) =
        strip_query_prefix_from_any(domain, &["domain name=", "domain name ", "domain "])
            .and_then(parse_maybe_quoted_query_text)
    {
        return Some(WindowSpawnTabDomain::DomainName(name));
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
        _ => None,
    }
}

fn spawn_command_in_new_window_from_query(query: &str) -> Option<WindowSpawnCommandQuery> {
    spawn_command_in_new_window_from_query_with_static_source(None, query)
}

fn spawn_command_in_new_window_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSpawnCommandQuery> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };
    spawn_command_table_query_from_lua_function_with_static_source(
        static_source,
        query,
        "spawncommandinnewwindow",
        true,
    )
    .or_else(|| {
        spawn_command_table_query_from_prefix_with_static_source(
            static_source,
            query,
            "spawn window=",
            true,
        )
    })
    .or_else(|| {
        spawn_command_table_query_from_prefix_with_static_source(
            static_source,
            query,
            "new window=",
            true,
        )
    })
    .or_else(|| {
        spawn_command_table_query_from_prefix_with_static_source(
            static_source,
            query,
            "spawncommandinnewwindow=",
            true,
        )
    })
    .or_else(|| spawn_command_query_from_prefix(query, "spawn window="))
    .or_else(|| spawn_command_query_from_prefix(query, "spawn window "))
    .or_else(|| spawn_command_query_from_prefix(query, "new window="))
    .or_else(|| spawn_command_query_from_prefix(query, "new window "))
    .or_else(|| spawn_command_query_from_prefix(query, "spawncommandinnewwindow="))
    .or_else(|| spawn_command_query_from_prefix(query, "spawncommandinnewwindow "))
}

fn spawn_command_options_in_new_window_from_query(
    query: &str,
) -> Option<WindowSpawnCommandQueryOptions> {
    spawn_command_options_in_new_window_from_query_with_static_source(None, query)
}

fn spawn_command_options_in_new_window_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSpawnCommandQueryOptions> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };
    spawn_command_table_options_from_lua_function_with_static_source(
        static_source,
        query,
        "spawncommandinnewwindow",
        true,
    )
    .or_else(|| {
        spawn_command_table_options_from_prefix_with_static_source(
            static_source,
            query,
            "spawn window=",
            true,
        )
    })
    .or_else(|| {
        spawn_command_table_options_from_prefix_with_static_source(
            static_source,
            query,
            "new window=",
            true,
        )
    })
    .or_else(|| {
        spawn_command_table_options_from_prefix_with_static_source(
            static_source,
            query,
            "spawncommandinnewwindow=",
            true,
        )
    })
    .or_else(|| spawn_command_options_from_prefix(query, "spawn window="))
    .or_else(|| spawn_command_options_from_prefix(query, "spawn window "))
    .or_else(|| spawn_command_options_from_prefix(query, "new window="))
    .or_else(|| spawn_command_options_from_prefix(query, "new window "))
    .or_else(|| spawn_command_options_from_prefix(query, "spawncommandinnewwindow="))
    .or_else(|| spawn_command_options_from_prefix(query, "spawncommandinnewwindow "))
}

fn spawn_command_table_options_from_prefix_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    prefix: &str,
    allow_position: bool,
) -> Option<WindowSpawnCommandQueryOptions> {
    let command = strip_query_table_assignment_from_prefix(query, prefix)?;
    spawn_command_table_options_from_query_with_static_source(
        static_source,
        command,
        allow_position,
    )
}

fn spawn_command_table_options_from_lua_function_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
    name: &str,
    allow_position: bool,
) -> Option<WindowSpawnCommandQueryOptions> {
    let command = strip_lua_function_call_from_query(query, name)?;
    spawn_command_table_options_from_query_with_static_source(
        static_source,
        command,
        allow_position,
    )
}

fn strip_query_table_assignment_from_prefix<'a>(query: &'a str, prefix: &str) -> Option<&'a str> {
    let query = query.trim();
    if let Some(name) = prefix.strip_suffix('=') {
        let candidate = query.get(..name.len())?;
        if !candidate.eq_ignore_ascii_case(name) {
            return None;
        }
        let rest = lua_trim_start_comments(query.get(name.len()..)?)?;
        if let Some(table_rest) = lua_trim_end_comments(rest)
            && table_rest.starts_with('{')
        {
            return Some(table_rest);
        }
        let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
        return lua_trim_end_comments(rest);
    }
    strip_query_prefix_from_any(query, &[prefix])
}

fn spawn_command_options_from_prefix(
    query: &str,
    prefix: &str,
) -> Option<WindowSpawnCommandQueryOptions> {
    let command = strip_query_prefix_from_any(query, &[prefix])?;
    let words = command_palette_query_words(command)?;
    let mut words = words.iter().map(String::as_str).peekable();
    let options = parse_spawn_command_query_options(&mut words).ok()?;
    if words.peek().is_some() {
        return None;
    }
    let has_options = options.cwd.is_some()
        || !options.environment.is_empty()
        || options.domain.is_some()
        || options.window_position.is_some();
    has_options.then_some(options)
}

fn split_horizontal_options_from_query(query: &str) -> Option<WindowSplitPaneOptions> {
    split_pane_options_from_query(query, "split horizontal=", SplitDirection::Right)
        .or_else(|| {
            split_pane_options_from_query(query, "split horizontal ", SplitDirection::Right)
        })
        .or_else(|| split_pane_options_from_query(query, "split right=", SplitDirection::Right))
        .or_else(|| split_pane_options_from_query(query, "split right ", SplitDirection::Right))
}

fn split_vertical_options_from_query(query: &str) -> Option<WindowSplitPaneOptions> {
    split_pane_options_from_query(query, "split vertical=", SplitDirection::Down)
        .or_else(|| split_pane_options_from_query(query, "split vertical ", SplitDirection::Down))
        .or_else(|| split_pane_options_from_query(query, "split down=", SplitDirection::Down))
        .or_else(|| split_pane_options_from_query(query, "split down ", SplitDirection::Down))
}

fn split_left_options_from_query(query: &str) -> Option<WindowSplitPaneOptions> {
    split_pane_options_from_query(query, "split left=", SplitDirection::Left)
        .or_else(|| split_pane_options_from_query(query, "split left ", SplitDirection::Left))
}

fn split_up_options_from_query(query: &str) -> Option<WindowSplitPaneOptions> {
    split_pane_options_from_query(query, "split up=", SplitDirection::Up)
        .or_else(|| split_pane_options_from_query(query, "split up ", SplitDirection::Up))
}

fn split_pane_action_name_options_from_query(query: &str) -> Option<WindowSplitPaneOptions> {
    split_pane_options_from_query(query, "splitpane right=", SplitDirection::Right)
        .or_else(|| split_pane_options_from_query(query, "splitpane right ", SplitDirection::Right))
        .or_else(|| {
            split_pane_options_from_query(
                query,
                "splitpane direction right=",
                SplitDirection::Right,
            )
        })
        .or_else(|| {
            split_pane_options_from_query(
                query,
                "splitpane direction right ",
                SplitDirection::Right,
            )
        })
        .or_else(|| {
            split_pane_options_from_query(
                query,
                "splitpane direction=right ",
                SplitDirection::Right,
            )
        })
        .or_else(|| split_pane_options_from_query(query, "splitpane down=", SplitDirection::Down))
        .or_else(|| split_pane_options_from_query(query, "splitpane down ", SplitDirection::Down))
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction down=", SplitDirection::Down)
        })
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction down ", SplitDirection::Down)
        })
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction=down ", SplitDirection::Down)
        })
        .or_else(|| split_pane_options_from_query(query, "splitpane left=", SplitDirection::Left))
        .or_else(|| split_pane_options_from_query(query, "splitpane left ", SplitDirection::Left))
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction left=", SplitDirection::Left)
        })
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction left ", SplitDirection::Left)
        })
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction=left ", SplitDirection::Left)
        })
        .or_else(|| split_pane_options_from_query(query, "splitpane up=", SplitDirection::Up))
        .or_else(|| split_pane_options_from_query(query, "splitpane up ", SplitDirection::Up))
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction up=", SplitDirection::Up)
        })
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction up ", SplitDirection::Up)
        })
        .or_else(|| {
            split_pane_options_from_query(query, "splitpane direction=up ", SplitDirection::Up)
        })
        .or_else(|| split_pane_structured_options_from_query(query))
}

fn split_pane_table_action_from_query(query: &str) -> Option<WindowSplitPaneOptions> {
    split_pane_table_action_from_query_with_static_source(None, query)
}

fn split_pane_table_action_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSplitPaneOptions> {
    let query = strip_wezterm_action_prefix(query).unwrap_or(query);
    let (rest, direction) = split_pane_table_from_lua_function(query, "splitpane", None)
        .or_else(|| {
            split_pane_table_from_lua_function(
                query,
                "splithorizontal",
                Some(SplitDirection::Right),
            )
        })
        .or_else(|| {
            split_pane_table_from_lua_function(query, "splitvertical", Some(SplitDirection::Down))
        })
        .or_else(|| {
            strip_query_table_assignment_from_prefix(query, "splitpane=").map(|rest| (rest, None))
        })
        .or_else(|| {
            strip_query_table_assignment_from_prefix(query, "splithorizontal=")
                .map(|rest| (rest, Some(SplitDirection::Right)))
        })
        .or_else(|| {
            strip_query_table_assignment_from_prefix(query, "splitvertical=")
                .map(|rest| (rest, Some(SplitDirection::Down)))
        })?;
    let rest = rest.trim();
    let resolved_rest;
    let rest = if rest.starts_with('{') {
        rest
    } else {
        let static_source = static_source?;
        resolved_rest = lua_table_insert_value_table_string_from_query(
            static_source.source,
            rest,
            static_source.max_start,
        )?;
        resolved_rest.as_str()
    };
    let table = rest.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let options = split_pane_table_options_from_query_with_static_source(static_source, table)?;
    let split_direction = direction.or(options.direction)?;
    let split_domain = options.domain.clone();
    let split_size = options.size;
    let split_top_level = options.top_level.unwrap_or(false);
    let (command, command_options) = options.split_command_parts()?;
    Some(WindowSplitPaneOptions {
        direction: split_direction,
        domain: split_domain,
        command,
        command_options,
        size: split_size,
        top_level: split_top_level,
    })
}

fn split_pane_table_from_lua_function<'a>(
    query: &'a str,
    name: &str,
    direction: Option<SplitDirection>,
) -> Option<(&'a str, Option<SplitDirection>)> {
    let table = strip_lua_function_call_from_query(query, name)?;
    Some((table, direction))
}

#[derive(Debug, Default)]
struct WindowSplitPaneTableOptions {
    direction: Option<SplitDirection>,
    domain: Option<WindowSpawnTabDomain>,
    command: Option<WindowSpawnCommandQuery>,
    spawn_args: Option<Vec<String>>,
    spawn_options: WindowSpawnCommandQueryOptions,
    spawn_label_seen: bool,
    size: Option<WindowSplitPaneSize>,
    top_level: Option<bool>,
}

impl WindowSplitPaneTableOptions {
    fn split_command_parts(
        self,
    ) -> Option<(
        Option<WindowSpawnCommandQuery>,
        Option<WindowSpawnCommandQueryOptions>,
    )> {
        if self.command.is_some()
            && (self.spawn_args.is_some()
                || self.spawn_options.cwd.is_some()
                || !self.spawn_options.environment.is_empty()
                || self.spawn_options.domain.is_some())
        {
            return None;
        }

        if let Some(mut args) = self.spawn_args {
            if args.is_empty() {
                return None;
            }
            let program = args.remove(0);
            return Some((
                Some(WindowSpawnCommandQuery {
                    label: None,
                    program,
                    args,
                    cwd: self.spawn_options.cwd,
                    environment: self.spawn_options.environment,
                    domain: self.spawn_options.domain,
                    window_position: None,
                }),
                None,
            ));
        }

        let command_options =
            split_pane_command_options_supported_without_program(&self.spawn_options)
                .then_some(self.spawn_options);
        Some((self.command, command_options))
    }
}

fn split_pane_table_options_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    table: &str,
) -> Option<WindowSplitPaneTableOptions> {
    let table = table.trim().trim_end_matches(',').trim();
    let mut options = WindowSplitPaneTableOptions::default();
    for field in split_lua_table_top_level_fields(table)? {
        split_pane_table_apply_field(static_source, field, &mut options)?;
    }
    Some(options)
}

fn split_pane_table_apply_field(
    static_source: Option<LuaStaticSource<'_>>,
    field: &str,
    options: &mut WindowSplitPaneTableOptions,
) -> Option<()> {
    let field = field.trim();
    if field.is_empty() {
        return Some(());
    }
    let (key, value) = split_lua_table_assignment_from_field(field)?;
    let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
    let value = value.trim().trim_end_matches(',').trim();
    if key.eq_ignore_ascii_case("direction") {
        if options.direction.is_some() {
            return None;
        }
        let value = parse_maybe_static_query_text(static_source, value)?;
        options.direction = Some(split_pane_direction_from_query(&value)?);
    } else if key.eq_ignore_ascii_case("domain") {
        if options.domain.is_some() {
            return None;
        }
        let value = parse_maybe_static_query_text(static_source, value)?;
        options.domain = Some(spawn_command_domain_from_query(&value)?);
    } else if key.eq_ignore_ascii_case("command") {
        if options.command.is_some()
            || options.spawn_args.is_some()
            || options.spawn_options.cwd.is_some()
            || !options.spawn_options.environment.is_empty()
            || options.spawn_options.domain.is_some()
        {
            return None;
        }
        options.command = Some(split_pane_table_command_from_query_with_static_source(
            static_source,
            value,
        )?);
    } else if key.eq_ignore_ascii_case("args") {
        if options.command.is_some() || options.spawn_args.is_some() {
            return None;
        }
        options.spawn_args = Some(split_lua_table_string_array_with_static_source(
            static_source,
            value,
        )?);
    } else if key.eq_ignore_ascii_case("cwd") {
        if options.command.is_some() || options.spawn_options.cwd.is_some() {
            return None;
        }
        let value = parse_maybe_static_query_text(static_source, value)?;
        options.spawn_options.cwd = Some(non_empty_spawn_command_option_value(&value).ok()?);
    } else if key.eq_ignore_ascii_case("label") {
        if options.command.is_some() || options.spawn_label_seen {
            return None;
        }
        let value = parse_maybe_static_query_text(static_source, value)?;
        let _ = non_empty_spawn_command_option_value(&value).ok()?;
        options.spawn_label_seen = true;
    } else if key.eq_ignore_ascii_case("set_environment_variables")
        || key.eq_ignore_ascii_case("set-environment-variables")
    {
        if options.command.is_some() || !options.spawn_options.environment.is_empty() {
            return None;
        }
        options.spawn_options.environment =
            split_lua_table_environment_from_query_with_static_source(static_source, value)?;
    } else if key.eq_ignore_ascii_case("size") {
        if options.size.is_some() {
            return None;
        }
        options.size = Some(split_pane_table_size_from_query_with_static_source(
            static_source,
            value,
        )?);
    } else if key.eq_ignore_ascii_case("top_level") || key.eq_ignore_ascii_case("top-level") {
        if options.top_level.is_some() {
            return None;
        }
        options.top_level = Some(parse_maybe_static_query_bool(static_source, value)?);
    } else {
        return None;
    }
    Some(())
}

fn split_pane_table_command_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowSpawnCommandQuery> {
    spawn_command_table_from_query_with_static_source(static_source, value, false)
}

fn spawn_command_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    allow_position: bool,
) -> Option<WindowSpawnCommandQuery> {
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
    let mut args = None;
    let mut cwd = None;
    let mut environment = BTreeMap::new();
    let mut domain = None;
    let mut window_position = None;
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
            if args.is_some() {
                return None;
            }
            args = Some(split_lua_table_string_array_with_static_source(
                static_source,
                value,
            )?);
        } else if key.eq_ignore_ascii_case("cwd") {
            if cwd.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            cwd = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("label") {
            if label.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            label = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("set_environment_variables")
            || key.eq_ignore_ascii_case("set-environment-variables")
        {
            if !environment.is_empty() {
                return None;
            }
            environment =
                split_lua_table_environment_from_query_with_static_source(static_source, value)?;
        } else if key.eq_ignore_ascii_case("domain") {
            if domain.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            domain = Some(spawn_command_domain_from_query(&value)?);
        } else if key.eq_ignore_ascii_case("position") {
            if !allow_position || window_position.is_some() {
                return None;
            }
            window_position = Some(
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
    let mut args = args?;
    if args.is_empty() {
        return None;
    }
    let program = args.remove(0);
    Some(WindowSpawnCommandQuery {
        label,
        program,
        args,
        cwd,
        environment,
        domain,
        window_position,
    })
}
