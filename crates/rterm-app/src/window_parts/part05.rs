fn lua_static_key_tables_variable_indexed_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<(String, LuaTableIndexedFieldAssignment<'a>)> {
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
    Some((
        key_table_name,
        LuaTableIndexedFieldAssignment {
            index,
            key,
            value: lua_top_level_statement_value_from_query(rest)?,
        },
    ))
}
fn lua_table_index_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
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
        value: lua_table_insert_value_table_string_from_query(source, rest, max_start)?,
    })
}

fn lua_u32_array_index_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<LuaTableIndexAssignment<&'a str>> {
    let after_open = lua_trim_start_comments(query)?.strip_prefix('[')?;
    let after_open = lua_trim_start_comments(after_open)?;
    let literal = lua_unsigned_integer_literal_from_query(after_open)?;
    let index = literal.parse().ok()?;
    let rest = lua_trim_start_comments(after_open.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableIndexAssignment {
        index,
        value: lua_table_insert_value_u32_from_query(source, rest, max_start)?,
    })
}

fn lua_string_array_index_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<LuaTableIndexAssignment<&'a str>> {
    let after_open = lua_trim_start_comments(query)?.strip_prefix('[')?;
    let after_open = lua_trim_start_comments(after_open)?;
    let literal = lua_unsigned_integer_literal_from_query(after_open)?;
    let index = literal.parse().ok()?;
    let rest = lua_trim_start_comments(after_open.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    Some(LuaTableIndexAssignment {
        index,
        value: lua_table_insert_value_string_from_query(source, rest, max_start)?,
    })
}

fn lua_table_length_append_assignment_value_after_target_from_query<'a>(
    source: &'a str,
    query: &'a str,
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
    lua_table_insert_value_table_string_from_query(source, rest, max_start)
}

fn lua_u32_array_length_append_assignment_value_after_target_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<&'a str> {
    let rest = lua_trim_start_comments(query)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('+')?)?;
    let literal = lua_unsigned_integer_literal_from_query(rest)?;
    if literal != "1" {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    lua_table_insert_value_u32_from_query(source, rest, max_start)
}

fn lua_string_array_length_append_assignment_value_after_target_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<&'a str> {
    let rest = lua_trim_start_comments(query)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('+')?)?;
    let literal = lua_unsigned_integer_literal_from_query(rest)?;
    if literal != "1" {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(']')?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('=')?)?;
    lua_table_insert_value_string_from_query(source, rest, max_start)
}

fn lua_static_nested_table_length_append_assignment_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
    variable: &str,
    key_table_name: &str,
) -> Option<LuaTableIndexOrAppendAssignment<String>> {
    let after_open = lua_trim_start_comments(query)?.strip_prefix('[')?;
    let after_hash = lua_trim_start_comments(after_open)?.strip_prefix('#')?;
    let after_hash = lua_trim_start_comments(after_hash)?;
    let after_variable = after_hash.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let (name, rest) = lua_nested_table_insert_key_from_query(source, after_variable, max_start)?;
    if name != key_table_name {
        return None;
    }
    Some(LuaTableIndexOrAppendAssignment {
        index: None,
        value: lua_table_length_append_assignment_value_after_target_from_query(
            source, rest, max_start,
        )?,
    })
}

fn lua_static_table_variable_index_or_append_assignment_from_query(
    source: &str,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexOrAppendAssignment<String>> {
    if let Some(assignment) =
        lua_static_table_variable_index_assignment_from_query(source, start, variable)
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
        value: lua_table_length_append_assignment_value_after_target_from_query(
            source, rest, start,
        )?,
    })
}

fn lua_static_table_variable_index_assignment_from_query(
    source: &str,
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
    lua_table_index_assignment_value_from_query(source, after_variable, start)
}

fn lua_static_table_variable_indexed_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    variable: &str,
) -> Option<LuaTableIndexedFieldAssignment<'a>> {
    let after_variable = source.get(start..)?.strip_prefix(variable)?;
    if after_variable
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    let after_variable = lua_trim_start_comments(after_variable)?;
    let (index, rest) = lua_table_array_index_access_rest_from_query(after_variable)?;
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

fn lua_static_table_variable_insert_append_value_from_query(
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

fn lua_static_string_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
) -> Option<&'a str> {
    if let Some(value) = lua_quoted_string_literal_from_query(query)
        .or_else(|| lua_long_bracket_literal_from_query(query))
    {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let max_start = lua_source_slice_start_offset(source, variable)?;
    lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)
}

fn lua_static_number_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
    mut literal_from_query: impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
    if let Some(value) = literal_from_query(query) {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let max_start = lua_source_slice_start_offset(source, variable)?;
    lua_static_number_variable_assignment_before_offset_from_query(
        source,
        variable,
        max_start,
        literal_from_query,
    )
}

fn lua_static_number_assignment_value_before_offset_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
    mut literal_from_query: impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
    if let Some(value) = literal_from_query(query) {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_number_variable_assignment_before_offset_from_query(
        source,
        variable,
        max_start,
        literal_from_query,
    )
}

fn lua_static_bool_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
) -> Option<&'a str> {
    if let Some(value) = lua_bool_literal_from_query(query) {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let max_start = lua_source_slice_start_offset(source, variable)?;
    lua_static_bool_variable_assignment_before_offset_from_query(source, variable, max_start)
}

fn parse_maybe_static_usize_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<usize> {
    if let Some(static_source) = static_source {
        if let Some(value) = lua_static_number_assignment_value_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
            lua_unsigned_integer_literal_from_query,
        ) {
            return value.parse::<usize>().ok();
        }
        parse_maybe_quoted_query_text(value)?.parse::<usize>().ok()
    } else {
        parse_maybe_quoted_query_text(value)?.parse::<usize>().ok()
    }
}

fn lua_static_bool_assignment_value_before_offset_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<&'a str> {
    if let Some(value) = lua_bool_literal_from_query(query) {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_bool_variable_assignment_before_offset_from_query(source, variable, max_start)
}

fn lua_static_easing_assignment_value_from_query<'a>(
    source: &'a str,
    query: &'a str,
) -> Option<&'a str> {
    if let Some(value) = lua_quoted_string_literal_from_query(query)
        .or_else(|| lua_long_bracket_literal_from_query(query))
        .or_else(|| lua_braced_table_literal_from_query(query))
    {
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let max_start = lua_source_slice_start_offset(source, variable)?;
    lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)
        .or_else(|| {
            lua_static_table_variable_assignment_before_offset_from_query(
                source, variable, max_start,
            )
        })
}

fn lua_static_action_assignment_value_before_offset_from_query<'a>(
    source: &'a str,
    query: &str,
    max_start: usize,
) -> Option<&'a str> {
    lua_static_expression_assignment_value_before_offset_from_query(source, query, max_start)
}

fn lua_static_expression_assignment_value_before_offset_from_query<'a>(
    source: &'a str,
    query: &str,
    max_start: usize,
) -> Option<&'a str> {
    let variable = lua_identifier_literal_from_query(query)?;
    let rest = query.get(variable.len()..)?;
    if lua_identifier_rest_has_expression_continuation_after_comment(rest) {
        return None;
    }
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_expression_variable_assignment_before_offset_from_query(source, variable, max_start)
}

fn lua_static_expression_variable_assignment_before_offset_from_query<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
) -> Option<&'a str> {
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
        if let Some(value) = lua_top_level_statement_value_from_query(value) {
            selected = Some(value);
        }
    }

    selected
}

fn lua_static_wezterm_action_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = query.trim_start();
    let alias = lua_identifier_literal_from_query(query)?;
    if !lua_static_wezterm_action_alias_before_offset(source, alias, max_start)? {
        return None;
    }

    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    let separator = match rest.chars().next()? {
        '.' | '[' => "",
        '{' | '(' => " ",
        _ => return None,
    };

    Some(format!("wezterm.action{separator}{rest}"))
}

fn lua_static_wezterm_action_alias_before_offset(
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
        selected = lua_static_wezterm_action_alias_value_from_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_action_alias_value_from_query(
    source: &str,
    max_start: usize,
    value: &str,
) -> bool {
    if lua_top_level_statement_value_from_query(value).is_some_and(|value| value.trim() == "act") {
        return true;
    }
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
    let Some((field, rest)) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource { source, max_start }),
        rest,
    ) else {
        return false;
    };
    field == "action" && lua_static_identifier_value_rest_is_statement_end(rest)
}

fn lua_static_wezterm_action_callback_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = query.trim_start();
    let alias = lua_identifier_literal_from_query(query)?;
    if !lua_static_wezterm_action_callback_alias_before_offset(source, alias, max_start)? {
        return None;
    }

    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    if !rest.starts_with('(') {
        return None;
    }

    Some(format!("wezterm.action_callback{rest}"))
}

fn lua_static_wezterm_action_callback_alias_before_offset(
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
        selected = lua_static_wezterm_action_callback_alias_value_from_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_action_callback_alias_value_from_query(
    source: &str,
    max_start: usize,
    value: &str,
) -> bool {
    if lua_top_level_statement_value_from_query(value)
        .is_some_and(|value| value.trim() == "action_callback")
    {
        return true;
    }
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
    let Some((field, rest)) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource { source, max_start }),
        rest,
    ) else {
        return false;
    };
    field == "action_callback" && lua_static_identifier_value_rest_is_statement_end(rest)
}

fn lua_static_wezterm_format_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = query.trim_start();
    let alias = lua_identifier_literal_from_query(query)?;
    if !lua_static_wezterm_format_alias_before_offset(source, alias, max_start)? {
        return None;
    }

    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    if !matches!(rest.chars().next()?, '(' | '{') {
        return None;
    }

    Some(format!("wezterm.format{rest}"))
}

fn lua_static_wezterm_format_alias_before_offset(
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
        selected = lua_static_wezterm_format_alias_value_from_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_format_alias_value_from_query(
    source: &str,
    max_start: usize,
    value: &str,
) -> bool {
    if value.trim() == "format" {
        return true;
    }
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
    let Some((field, rest)) = lua_table_map_field_key_from_query_with_static_source(
        Some(LuaStaticSource { source, max_start }),
        rest,
    ) else {
        return false;
    };
    field == "format" && lua_static_identifier_value_rest_is_statement_end(rest)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_top_level_statement_value_from_query(value: &str) -> Option<&str> {
    let value = lua_trim_start_comments(value)?.trim_start();
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut table_depth = 0usize;
    let mut paren_depth = 0usize;

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
            if table_depth == 0 && paren_depth == 0 && lua_block_depth == 0 {
                let candidate = value[..index].trim();
                return (!candidate.is_empty()).then_some(candidate);
            }
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
            '[' => {
                if let Some((content_start, closing)) =
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
            }
            '{' => {
                table_depth = table_depth.saturating_add(1);
                continue;
            }
            '}' => {
                table_depth = table_depth.saturating_sub(1);
                continue;
            }
            '(' => {
                paren_depth = paren_depth.saturating_add(1);
                continue;
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                continue;
            }
            '\n' | ';' if table_depth == 0 && paren_depth == 0 && lua_block_depth == 0 => {
                let candidate = value[..index].trim();
                return (!candidate.is_empty()).then_some(candidate);
            }
            _ => {}
        }

        if lua_source_keyword_at(value, index, "function")
            || lua_source_keyword_at(value, index, "then")
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

    let candidate = value.trim();
    (!candidate.is_empty()).then_some(candidate)
}

fn lua_static_identifier_value_rest_is_statement_end(rest: &str) -> bool {
    for character in rest.chars() {
        match character {
            ' ' | '\t' | '\r' => {}
            '\n' | ';' => return true,
            '-' => return rest.trim_start().starts_with("--"),
            _ => return false,
        }
    }
    true
}

fn lua_static_string_variable_assignment_before_offset_from_query<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
) -> Option<&'a str> {
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
        if let Some(value) = lua_quoted_string_literal_from_query(value)
            .or_else(|| lua_long_bracket_literal_from_query(value))
        {
            selected = Some(value);
        }
    }

    selected
}

fn lua_static_bool_variable_assignment_before_offset_from_query<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
) -> Option<&'a str> {
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
        if let Some(value) = lua_bool_literal_from_query(value) {
            selected = Some(value);
        }
    }

    selected
}

fn lua_static_number_variable_assignment_before_offset_from_query<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
    mut literal_from_query: impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
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
        if let Some(value) = literal_from_query(value) {
            selected = Some(value);
        }
    }

    selected
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_static_table_variable_assignment_before_offset_from_query<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
) -> Option<&'a str> {
    let source = source.get(..max_start)?;
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut table_depth = 0usize;
    let mut selected = None;

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
            '[' => {
                if let Some((content_start, closing)) =
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
            }
            '{' => {
                table_depth = table_depth.saturating_add(1);
                continue;
            }
            '}' => {
                table_depth = table_depth.saturating_sub(1);
                continue;
            }
            _ => {}
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

        if lua_block_depth == 0
            && table_depth == 0
            && lua_source_index_starts_statement(source, index)
        {
            let rest = if lua_source_keyword_at(source, index, "local") {
                lua_trim_start_comments(source.get(index + "local".len()..)?)?
            } else {
                source.get(index..)?
            };
            if let Some(table) =
                lua_static_table_variable_assignment_table_from_query(rest, variable)
            {
                selected = Some(table);
            }
        }
    }

    selected
}

fn lua_source_index_starts_statement(source: &str, index: usize) -> bool {
    for character in source[..index].chars().rev() {
        if matches!(character, '\n' | '\r') {
            return true;
        }
        if character.is_whitespace() {
            continue;
        }
        return character == ';';
    }
    true
}

fn lua_complete_label_len_from_query(query: &str) -> Option<usize> {
    let rest = query.strip_prefix("::")?;
    let rest = lua_trim_start_comments(rest)?;
    let label = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(label.len()..)?)?;
    let rest = rest.strip_prefix("::")?;
    Some(query.len().saturating_sub(rest.len()))
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_top_level_statement_start_indices_before_offset(
    source: &str,
    max_start: usize,
) -> Option<Vec<usize>> {
    let source = source.get(..max_start)?;
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut label_end = None;
    let mut lua_block_depth = 0usize;
    let mut table_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut starts = Vec::new();

    for (index, character) in source.char_indices() {
        if let Some(end) = label_end {
            if index < end {
                continue;
            }
            label_end = None;
        }
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
            '[' => {
                if let Some((content_start, closing)) =
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
            }
            '{' => {
                table_depth = table_depth.saturating_add(1);
                continue;
            }
            '}' => {
                table_depth = table_depth.saturating_sub(1);
                continue;
            }
            '(' => {
                paren_depth = paren_depth.saturating_add(1);
                continue;
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                continue;
            }
            _ => {}
        }

        if lua_block_depth == 0
            && table_depth == 0
            && paren_depth == 0
            && source[index..].starts_with("::")
            && let Some(label_len) = lua_complete_label_len_from_query(&source[index..])
        {
            label_end = Some(index + label_len);
            if starts.last().copied() != Some(index) {
                starts.push(index);
            }
            let after_label = source.get(index + label_len..)?;
            let after_label = lua_trim_start_comments(after_label)?;
            if !after_label.is_empty() {
                let after_label_start = source.len().saturating_sub(after_label.len());
                if starts.last().copied() != Some(after_label_start) {
                    starts.push(after_label_start);
                }
            }
            continue;
        }

        if lua_source_keyword_at(source, index, "elseif") {
            lua_block_depth = lua_block_depth.saturating_sub(1);
            continue;
        }

        if lua_source_keyword_at(source, index, "function") {
            if lua_block_depth == 0
                && table_depth == 0
                && paren_depth == 0
                && !character.is_whitespace()
                && lua_source_index_starts_statement(source, index)
                && starts.last().copied() != Some(index)
            {
                starts.push(index);
            }
            lua_block_depth = lua_block_depth.saturating_add(1);
            continue;
        }

        if lua_source_keyword_at(source, index, "then")
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

        if lua_block_depth == 0
            && table_depth == 0
            && paren_depth == 0
            && !character.is_whitespace()
            && lua_source_index_starts_statement(source, index)
            && starts.last().copied() != Some(index)
        {
            starts.push(index);
        }
    }

    Some(starts)
}

fn lua_config_field_access_rest_from_query_with_static_key<'a>(
    source: &'a str,
    query: &'a str,
    field: &str,
    max_start: usize,
) -> Option<&'a str> {
    let query = lua_trim_start_comments(query)?;
    if let Some(rest) = query.strip_prefix('.') {
        let rest = lua_trim_start_comments(rest)?;
        if !rest.starts_with(field) || !lua_config_assignment_field_has_boundaries(rest, 0, field) {
            return None;
        }
        return rest.get(field.len()..);
    }

    let after_open = lua_trim_start_comments(query.strip_prefix('[')?)?;
    let (key, after_key) =
        lua_config_bracket_assignment_key_from_query(source, after_open, max_start)?;
    if key != field {
        return None;
    }

    lua_trim_start_comments(after_key)?.strip_prefix(']')
}

fn lua_config_nested_table_insert_append_from_query(
    source: &str,
    start: usize,
    receiver: &str,
    field: &str,
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
    let after_receiver = lua_config_receiver_prefix_rest(rest, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (name, rest) = lua_nested_table_insert_key_from_query(source, rest, start)?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    if let Some(value) =
        lua_table_insert_argument_value_table_string_from_query(source, rest, start)
    {
        return Some((
            name,
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
        name,
        LuaTableInsertValue {
            position: Some(position),
            value: lua_table_insert_argument_value_table_string_from_query(source, rest, start)?,
        },
    ))
}

fn lua_config_nested_key_table_index_or_append_assignment_from_query(
    source: &str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<(String, LuaTableIndexOrAppendAssignment<String>)> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (name, rest) = lua_nested_table_insert_key_from_query(source, rest, start)?;
    if let Some(assignment) = lua_table_index_assignment_value_from_query(source, rest, start) {
        return Some((
            name,
            LuaTableIndexOrAppendAssignment {
                index: Some(assignment.index),
                value: assignment.value,
            },
        ));
    }

    let assignment = lua_config_nested_table_length_append_assignment_from_query(
        source, rest, start, receiver, field, &name,
    )?;
    Some((name, assignment))
}

fn lua_config_nested_key_table_indexed_field_assignment_from_query<'a>(
    source: &'a str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<(String, LuaTableIndexedFieldAssignment<'a>)> {
    let after_receiver = lua_config_receiver_prefix_rest(source.get(start..)?, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (name, rest) = lua_nested_table_insert_key_from_query(source, rest, start)?;
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
    Some((
        name,
        LuaTableIndexedFieldAssignment {
            index,
            key,
            value: lua_top_level_statement_value_from_query(rest)?,
        },
    ))
}

fn lua_config_nested_table_length_append_assignment_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
    receiver: &str,
    field: &str,
    key_table_name: &str,
) -> Option<LuaTableIndexOrAppendAssignment<String>> {
    let after_open = lua_trim_start_comments(query)?.strip_prefix('[')?;
    let after_hash = lua_trim_start_comments(after_open)?.strip_prefix('#')?;
    let after_hash = lua_trim_start_comments(after_hash)?;
    let after_receiver = lua_config_receiver_prefix_rest(after_hash, receiver)?;
    let after_receiver = lua_trim_start_comments(after_receiver)?;
    let rest = lua_config_field_access_rest_from_query_with_static_key(
        source,
        after_receiver,
        field,
        max_start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (name, rest) = lua_nested_table_insert_key_from_query(source, rest, max_start)?;
    if name != key_table_name {
        return None;
    }
    Some(LuaTableIndexOrAppendAssignment {
        index: None,
        value: lua_table_length_append_assignment_value_after_target_from_query(
            source, rest, max_start,
        )?,
    })
}

fn lua_nested_table_insert_key_from_query<'a>(
    source: &str,
    query: &'a str,
    max_start: usize,
) -> Option<(String, &'a str)> {
    let query = lua_trim_start_comments(query)?;
    if let Some(rest) = query.strip_prefix('.') {
        let rest = lua_trim_start_comments(rest)?;
        let name = lua_identifier_literal_from_query(rest)?;
        return Some((name.to_owned(), rest.get(name.len()..)?));
    }

    let after_open = lua_trim_start_comments(query.strip_prefix('[')?)?;
    if let Some(key_literal) = lua_quoted_string_literal_from_query(after_open)
        .or_else(|| lua_long_bracket_literal_from_query(after_open))
    {
        let key = parse_maybe_quoted_query_text(key_literal)?;
        let rest = lua_trim_start_comments(after_open.get(key_literal.len()..)?)?;
        return Some((key, rest.strip_prefix(']')?));
    }

    let variable = lua_identifier_literal_from_query(after_open)?;
    let rest = lua_trim_start_comments(after_open.get(variable.len()..)?)?;
    let rest = rest.strip_prefix(']')?;
    let key =
        lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)
            .and_then(parse_maybe_quoted_query_text)?;
    let key = non_empty_spawn_command_option_value(&key).ok()?;
    Some((key, rest))
}

fn lua_table_with_inserted_field(
    table: Option<String>,
    position: Option<usize>,
    field: &str,
) -> Option<String> {
    let Some(table) = table else {
        if position.is_some_and(|position| position != 1) {
            return None;
        }
        return Some(format!("{{ {field} }}"));
    };
    let table_fields = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut fields = if table_fields.is_empty() {
        Vec::new()
    } else {
        split_lua_table_top_level_fields(table_fields)?
            .into_iter()
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .map(str::to_owned)
            .collect()
    };

    match position {
        Some(position) => {
            if position == 0 || position > fields.len() + 1 {
                return None;
            }
            fields.insert(position - 1, field.to_owned());
        }
        None => fields.push(field.to_owned()),
    }

    Some(format!("{{ {} }}", fields.join(",\n")))
}

fn lua_key_tables_with_inserted_assignment(
    key_tables: Option<String>,
    key_table_name: &str,
    position: Option<usize>,
    assignment: &str,
) -> Option<String> {
    let Some(key_tables) = key_tables else {
        if position.is_some_and(|position| position != 1) {
            return None;
        }
        return Some(format!(
            "{{ {} = {{ {assignment} }} }}",
            lua_table_key_from_text(key_table_name)
        ));
    };

    let table = key_tables
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let mut fields = Vec::new();
    let mut appended = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }

        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            fields.push(field.to_owned());
            continue;
        };
        let Some(name) = split_lua_table_key_from_query(key.trim()) else {
            fields.push(field.to_owned());
            continue;
        };
        if name == key_table_name {
            fields.push(format!(
                "{} = {}",
                key.trim(),
                lua_table_with_inserted_field(Some(value.trim().to_owned()), position, assignment)?
            ));
            appended = true;
        } else {
            fields.push(field.to_owned());
        }
    }

    if !appended {
        if position.is_some_and(|position| position != 1) {
            return None;
        }
        fields.push(format!(
            "{} = {{ {assignment} }}",
            lua_table_key_from_text(key_table_name)
        ));
    }

    Some(format!("{{ {} }}", fields.join(",\n")))
}

fn lua_key_tables_with_index_or_append_assigned_assignment(
    key_tables: Option<String>,
    key_table_name: &str,
    index: Option<usize>,
    assignment: &str,
) -> Option<String> {
    let Some(key_tables) = key_tables else {
        return Some(format!(
            "{{ {} = {} }}",
            lua_table_key_from_text(key_table_name),
            lua_table_with_index_or_append_assigned_field(None, index, assignment)?
        ));
    };

    let table = key_tables
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let mut fields = Vec::new();
    let mut assigned = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }

        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            fields.push(field.to_owned());
            continue;
        };
        let Some(name) = split_lua_table_key_from_query(key.trim()) else {
            fields.push(field.to_owned());
            continue;
        };
        if name == key_table_name {
            fields.push(format!(
                "{} = {}",
                key.trim(),
                lua_table_with_index_or_append_assigned_field(
                    Some(value.trim().to_owned()),
                    index,
                    assignment
                )?
            ));
            assigned = true;
        } else {
            fields.push(field.to_owned());
        }
    }

    if !assigned {
        fields.push(format!(
            "{} = {}",
            lua_table_key_from_text(key_table_name),
            lua_table_with_index_or_append_assigned_field(None, index, assignment)?
        ));
    }

    Some(format!("{{ {} }}", fields.join(",\n")))
}

fn lua_key_tables_with_index_field_assigned(
    key_tables: Option<String>,
    key_table_name: &str,
    index: usize,
    assignment_key: &str,
    assignment_value: &str,
) -> Option<String> {
    let key_tables = key_tables?;
    let table = key_tables
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let mut fields = Vec::new();
    let mut assigned = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }

        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            fields.push(field.to_owned());
            continue;
        };
        let Some(name) = split_lua_table_key_from_query(key.trim()) else {
            fields.push(field.to_owned());
            continue;
        };
        if name == key_table_name {
            fields.push(format!(
                "{} = {}",
                key.trim(),
                lua_table_with_index_field_assigned(
                    Some(value.trim().to_owned()),
                    index,
                    assignment_key,
                    assignment_value
                )?
            ));
            assigned = true;
        } else {
            fields.push(field.to_owned());
        }
    }

    assigned.then(|| format!("{{ {} }}", fields.join(",\n")))
}

fn lua_table_with_index_or_append_assigned_field(
    table: Option<String>,
    index: Option<usize>,
    field: &str,
) -> Option<String> {
    if index.is_some_and(|index| index == 0) {
        return None;
    }

    let Some(table) = table else {
        if index.is_some_and(|index| index != 1) {
            return None;
        }
        return Some(format!("{{ {field} }}"));
    };

    let table_fields = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut implicit_fields = Vec::new();
    let mut indexed_fields = BTreeMap::new();

    for table_field in split_lua_table_top_level_fields(table_fields)? {
        let table_field = table_field.trim();
        if table_field.is_empty() {
            continue;
        }

        if let Some((key, value)) = split_lua_table_assignment_from_field(table_field)
            && let Some(existing_index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !implicit_fields.is_empty()
                || existing_index == 0
                || indexed_fields.contains_key(&existing_index)
            {
                return None;
            }
            indexed_fields.insert(existing_index, value.trim().to_owned());
            continue;
        }

        if !indexed_fields.is_empty() {
            return None;
        }
        implicit_fields.push(table_field.to_owned());
    }

    let mut fields = if indexed_fields.is_empty() {
        implicit_fields
    } else {
        let mut fields = Vec::new();
        for existing_index in 1..=indexed_fields.len() {
            fields.push(indexed_fields.remove(&existing_index)?);
        }
        fields
    };

    let index = index.unwrap_or(fields.len() + 1);
    if index > fields.len() + 1 {
        return None;
    }
    if index <= fields.len() {
        field.clone_into(&mut fields[index - 1]);
    } else {
        fields.push(field.to_owned());
    }

    Some(format!("{{ {} }}", fields.join(",\n")))
}

fn lua_table_with_index_field_assigned(
    table: Option<String>,
    index: usize,
    key: &str,
    value: &str,
) -> Option<String> {
    if index == 0 {
        return None;
    }

    let table = table?;
    let table_fields = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut implicit_fields = Vec::new();
    let mut indexed_fields = BTreeMap::new();

    for table_field in split_lua_table_top_level_fields(table_fields)? {
        let table_field = table_field.trim();
        if table_field.is_empty() {
            continue;
        }

        if let Some((field_key, field_value)) = split_lua_table_assignment_from_field(table_field)
            && let Some(existing_index) = split_lua_table_array_index_from_query(field_key.trim())
        {
            if !implicit_fields.is_empty()
                || existing_index == 0
                || indexed_fields.contains_key(&existing_index)
            {
                return None;
            }
            indexed_fields.insert(existing_index, field_value.trim().to_owned());
            continue;
        }

        if !indexed_fields.is_empty() {
            return None;
        }
        implicit_fields.push(table_field.to_owned());
    }

    let mut fields = if indexed_fields.is_empty() {
        implicit_fields
    } else {
        let mut fields = Vec::new();
        for existing_index in 1..=indexed_fields.len() {
            fields.push(indexed_fields.remove(&existing_index)?);
        }
        fields
    };

    let existing = fields.get_mut(index - 1)?;
    *existing = lua_table_with_assigned_field(Some(existing.clone()), key, value)?;

    Some(format!("{{ {} }}", fields.join(",\n")))
}

fn lua_table_with_assigned_field(table: Option<String>, key: &str, value: &str) -> Option<String> {
    let field = format!("{} = {}", lua_table_key_from_text(key), value.trim());
    let Some(table) = table else {
        return Some(format!("{{ {field} }}"));
    };

    let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut fields = Vec::new();
    let mut assigned = false;

    for existing in split_lua_table_top_level_fields(table)? {
        let existing = existing.trim();
        if existing.is_empty() {
            continue;
        }
        let Some((existing_key, _existing_value)) = split_lua_table_assignment_from_field(existing)
        else {
            fields.push(existing.to_owned());
            continue;
        };
        let Some(existing_key) = split_lua_table_key_from_query(existing_key.trim()) else {
            fields.push(existing.to_owned());
            continue;
        };
        if existing_key == key {
            if !assigned {
                fields.push(field.clone());
                assigned = true;
            }
        } else {
            fields.push(existing.to_owned());
        }
    }

    if !assigned {
        fields.push(field);
    }

    Some(format!("{{ {} }}", fields.join(",\n")))
}

fn lua_key_tables_with_assigned_table(
    key_tables: Option<String>,
    key_table_name: &str,
    assignments: &str,
) -> Option<String> {
    let field = format!(
        "{} = {}",
        lua_table_key_from_text(key_table_name),
        assignments.trim()
    );
    let Some(key_tables) = key_tables else {
        return Some(format!("{{ {field} }}"));
    };

    let table = key_tables
        .trim()
        .strip_prefix('{')?
        .strip_suffix('}')?
        .trim();
    let mut fields = Vec::new();
    let mut assigned = false;

    for existing in split_lua_table_top_level_fields(table)? {
        let existing = existing.trim();
        if existing.is_empty() {
            continue;
        }
        let Some((key, _value)) = split_lua_table_assignment_from_field(existing) else {
            fields.push(existing.to_owned());
            continue;
        };
        let Some(name) = split_lua_table_key_from_query(key.trim()) else {
            fields.push(existing.to_owned());
            continue;
        };
        if name == key_table_name {
            if !assigned {
                fields.push(field.clone());
                assigned = true;
            }
        } else {
            fields.push(existing.to_owned());
        }
    }

    if !assigned {
        fields.push(field);
    }

    Some(format!("{{ {} }}", fields.join(",\n")))
}

fn lua_table_key_from_text(key: &str) -> String {
    if lua_identifier_literal_from_query(key).is_some_and(|identifier| identifier == key) {
        return key.to_owned();
    }

    format!("[\"{}\"]", key.replace('\\', "\\\\").replace('"', "\\\""))
}

struct NativeLoadSchemeColorsAssignment {
    path: String,
    variable: Option<NativeLoadSchemeVariableReference>,
}

struct NativeBuiltinColorSchemeAssignment {
    name: String,
    variable: Option<NativeLoadSchemeVariableReference>,
}

enum NativeConfigColorsLuaSource<'a> {
    Table {
        colors: &'a str,
        variable: Option<NativeLoadSchemeVariableReference>,
    },
    LoadScheme(NativeLoadSchemeColorsAssignment),
    Builtin(NativeBuiltinColorSchemeAssignment),
    DefaultColors {
        variable: Option<NativeLoadSchemeVariableReference>,
    },
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_config_colors_source_from_query(
    source: &str,
) -> Option<Option<NativeConfigColorsLuaSource<'_>>> {
    if let Some(table) = lua_config_static_return_table_from_query(source) {
        let mut literal_from_query = lua_color_variable_mutation_value_literal_from_query;
        let Some(colors) =
            lua_config_table_field_assignment_from_query(table, "colors", &mut literal_from_query)
        else {
            return Some(None);
        };
        return Some(Some(lua_config_colors_source_value_from_query(
            source, colors,
        )?));
    }
    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");

    if let Some(source) = lua_config_colors_direct_source_from_query(source, receiver)? {
        return Some(Some(source));
    }

    if let Some(colors) = lua_config_table_assignment_from_query(source, "colors") {
        return Some(Some(NativeConfigColorsLuaSource::Table {
            colors,
            variable: None,
        }));
    }

    if let Some(load_scheme) = lua_config_load_scheme_colors_assignment_from_query(source) {
        return Some(Some(NativeConfigColorsLuaSource::LoadScheme(load_scheme)));
    }

    Some(None)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_config_colors_direct_source_from_query<'a>(
    source: &'a str,
    receiver: &str,
) -> Option<Option<NativeConfigColorsLuaSource<'a>>> {
    let mut selected = None;
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;

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

        if source[index..].starts_with("colors")
            && lua_config_assignment_field_has_boundaries(source, index, "colors")
            && lua_config_dot_assignment_has_receiver(source, index, receiver)
            && lua_block_depth == 0
        {
            let rest = lua_trim_start_comments(source.get(index + "colors".len()..)?)?;
            if let Some(rest) = rest.strip_prefix('=')
                && let Some(source) = lua_config_colors_source_value_from_query(
                    source,
                    lua_trim_start_comments(rest)?,
                )
            {
                selected = Some(source);
            }
        }

        if character == '['
            && lua_block_depth == 0
            && let Some(rest) =
                lua_config_bracket_assignment_rest_from_query(source, index, receiver, "colors")
            && let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('=')
            && let Some(source) =
                lua_config_colors_source_value_from_query(source, lua_trim_start_comments(rest)?)
        {
            selected = Some(source);
        }
    }

    Some(selected)
}

fn lua_config_colors_source_value_from_query<'a>(
    source: &'a str,
    value: &'a str,
) -> Option<NativeConfigColorsLuaSource<'a>> {
    let value = lua_trim_start_comments(value)?.trim_start();
    if value.starts_with('{') {
        let colors = lua_braced_table_literal_from_query(value)?;
        return Some(NativeConfigColorsLuaSource::Table {
            colors,
            variable: None,
        });
    }

    if let Some(path) =
        lua_wezterm_color_load_scheme_path_from_query_with_static_source(source, value)
    {
        return Some(NativeConfigColorsLuaSource::LoadScheme(
            NativeLoadSchemeColorsAssignment {
                path,
                variable: None,
            },
        ));
    }
    if let Some(name) =
        lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(source, value)
    {
        return Some(NativeConfigColorsLuaSource::Builtin(
            NativeBuiltinColorSchemeAssignment {
                name,
                variable: None,
            },
        ));
    }

    if let Some(name) = lua_whole_map_builtin_color_scheme_name_from_query(source, value) {
        return Some(NativeConfigColorsLuaSource::Builtin(
            NativeBuiltinColorSchemeAssignment {
                name,
                variable: None,
            },
        ));
    }
    if lua_wezterm_default_colors_from_query_with_static_source(source, value).is_some() {
        return Some(NativeConfigColorsLuaSource::DefaultColors { variable: None });
    }

    let variable = lua_identifier_literal_from_query(value)?;
    let reference_start = lua_source_slice_start_offset(source, variable)?;
    lua_config_colors_variable_source_before_offset(source, variable, reference_start)
}

fn lua_whole_map_builtin_color_scheme_name_from_query(source: &str, query: &str) -> Option<String> {
    let variable = lua_identifier_literal_from_query(query)?;
    let index = query.get(variable.len()..)?;
    if !lua_trim_start_comments(index)?.starts_with('[') {
        return None;
    }
    let reference_start = lua_source_slice_start_offset(source, query)?;
    let (value, binding_start) =
        lua_static_builtin_scheme_binding_before_offset(source, variable, reference_start)?;
    if !lua_static_builtin_scheme_map_identity_is_safe_for_lookup(
        source,
        variable,
        binding_start,
        reference_start,
    )? {
        return None;
    }
    let canonical =
        lua_static_wezterm_builtin_color_scheme_call_query_from_query(source, value, binding_start)
            .or_else(|| {
                lua_static_wezterm_builtin_color_scheme_alias_query_from_query(
                    source,
                    value,
                    binding_start,
                )
            })?;
    let combined = format!("{canonical}{index}");
    lua_wezterm_builtin_color_scheme_name_from_call_query(source, &combined, reference_start)
}

fn lua_static_builtin_scheme_map_identity_is_safe_for_lookup(
    source: &str,
    variable: &str,
    binding_start: usize,
    lookup_start: usize,
) -> Option<bool> {
    let starts = lua_top_level_statement_start_indices_before_offset(source, source.len())?;
    let lookup_statement_index = starts.iter().rposition(|start| *start <= lookup_start)?;
    let lookup_statement_start = *starts.get(lookup_statement_index)?;
    let lookup_statement_end = starts
        .get(lookup_statement_index + 1)
        .copied()
        .unwrap_or(source.len());
    let mut capturing_functions: Vec<String> = Vec::new();

    for (index, start) in starts
        .iter()
        .copied()
        .enumerate()
        .take(lookup_statement_index)
    {
        if start <= binding_start {
            continue;
        }
        let end = starts
            .get(index + 1)
            .copied()
            .unwrap_or(lookup_statement_start)
            .min(lookup_statement_start);
        let statement = source.get(start..end)?;
        if !lua_static_builtin_scheme_statement_preserves_map_identity(
            source,
            statement,
            start,
            variable,
            &mut capturing_functions,
        )? {
            return Some(false);
        }
    }

    let lookup_statement = source.get(lookup_statement_start..lookup_statement_end)?;
    for captured in &capturing_functions {
        if lua_static_query_contains_identifier(lookup_statement, captured)? {
            return Some(false);
        }
    }
    if !lua_static_builtin_scheme_statement_is_readonly_config_consumer(
        source,
        lookup_statement,
        lookup_statement_start,
        variable,
    )? && !lua_static_builtin_scheme_statement_is_readonly_palette_binding(
        source,
        lookup_statement,
        lookup_statement_start,
        variable,
    )? {
        return Some(false);
    }

    for (index, start) in starts
        .iter()
        .copied()
        .enumerate()
        .skip(lookup_statement_index + 1)
    {
        let end = starts.get(index + 1).copied().unwrap_or(source.len());
        let statement = source.get(start..end)?;
        if let Some(rebind_is_safe) = lua_static_builtin_scheme_map_rebind_is_safe(
            source,
            statement,
            start,
            variable,
            &capturing_functions,
        )? {
            return Some(rebind_is_safe);
        }
        if !lua_static_builtin_scheme_statement_preserves_map_identity(
            source,
            statement,
            start,
            variable,
            &mut capturing_functions,
        )? {
            return Some(false);
        }
    }

    Some(true)
}

fn lua_static_builtin_scheme_statement_preserves_map_identity(
    source: &str,
    statement: &str,
    statement_start: usize,
    variable: &str,
    capturing_functions: &mut Vec<String>,
) -> Option<bool> {
    if let Some(function) = lua_static_builtin_scheme_function_definition_name(statement)? {
        if lua_static_builtin_scheme_fragment_references_map_identity(
            statement,
            variable,
            capturing_functions,
        )? {
            capturing_functions.push(function);
        }
        return Some(true);
    }
    for captured in capturing_functions.iter() {
        if lua_static_query_contains_identifier(statement, captured)? {
            return Some(false);
        }
    }
    if !lua_static_query_contains_identifier(statement, variable)? {
        return Some(true);
    }
    Some(
        lua_static_builtin_scheme_statement_is_readonly_config_consumer(
            source,
            statement,
            statement_start,
            variable,
        )? || lua_static_builtin_scheme_statement_is_readonly_palette_binding(
            source,
            statement,
            statement_start,
            variable,
        )?,
    )
}

fn lua_static_builtin_scheme_statement_is_readonly_config_consumer(
    source: &str,
    statement: &str,
    statement_start: usize,
    variable: &str,
) -> Option<bool> {
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
    let statement = normalized.trim_start();
    let Some((targets, value)) = split_lua_static_load_scheme_path_assignment_statement(statement)
    else {
        return Some(false);
    };
    let targets = split_lua_top_level_arguments(targets)?;
    let [target] = targets.as_slice() else {
        return Some(false);
    };
    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let Some(rest) = lua_config_receiver_prefix_rest(target.trim(), receiver) else {
        return Some(false);
    };
    let Some(rest) = rest.trim_start().strip_prefix('.') else {
        return Some(false);
    };

    if let Some(rest) = rest.strip_prefix("colors")
        && !rest.chars().next().is_some_and(is_lua_identifier_character)
        && rest.trim().is_empty()
    {
        return lua_static_builtin_scheme_map_lookup_is_exact(
            source,
            value,
            statement_start,
            variable,
        );
    }

    let Some(rest) = rest.strip_prefix("color_schemes") else {
        return Some(false);
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return Some(false);
    }
    if rest.trim().is_empty() {
        return lua_static_builtin_scheme_map_table_consumer_is_readonly(
            source,
            value,
            statement_start,
            variable,
        );
    }
    let Some((_, tail)) = color_scheme_lua_table_assignment_key_from_query(rest) else {
        return Some(false);
    };
    if !tail.trim().is_empty() {
        return Some(false);
    }
    lua_static_builtin_scheme_map_lookup_is_exact(source, value, statement_start, variable)
}

fn lua_static_builtin_scheme_statement_is_readonly_palette_binding(
    source: &str,
    statement: &str,
    statement_start: usize,
    variable: &str,
) -> Option<bool> {
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
    let statement = normalized.trim_start();
    let statement = if lua_source_keyword_at(statement, 0, "local") {
        statement.get("local".len()..)?.trim_start()
    } else {
        statement
    };
    let Some((targets, value)) = split_lua_static_load_scheme_path_assignment_statement(statement)
    else {
        return Some(false);
    };
    let targets = split_lua_top_level_arguments(targets)?;
    let [target] = targets.as_slice() else {
        return Some(false);
    };
    let Some(target) = lua_static_load_scheme_path_assignment_target_identifier(target) else {
        return Some(false);
    };
    if target == variable {
        return Some(false);
    }
    lua_static_builtin_scheme_map_lookup_is_exact(source, value, statement_start, variable)
}

fn lua_static_builtin_scheme_map_table_consumer_is_readonly(
    source: &str,
    value: &str,
    statement_start: usize,
    variable: &str,
) -> Option<bool> {
    let value = lua_trim_start_comments(value)?;
    let Some(table) = lua_braced_table_literal_from_query(value) else {
        return Some(false);
    };
    if !lua_static_builtin_scheme_tail_is_statement_end(value.get(table.len()..)?)? {
        return Some(false);
    }
    let fields = table.trim().strip_prefix('{')?.strip_suffix('}')?;
    for field in split_lua_table_top_level_fields(fields)? {
        let field = lua_trim_start_comments(field)?.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, field_value)) = split_lua_table_assignment_from_field(field) else {
            return Some(false);
        };
        if !lua_static_query_contains_identifier(field, variable)? {
            continue;
        }
        if lua_static_query_contains_identifier(key, variable)?
            || !lua_static_builtin_scheme_map_lookup_is_exact(
                source,
                field_value,
                statement_start,
                variable,
            )?
        {
            return Some(false);
        }
    }
    Some(true)
}

fn lua_static_builtin_scheme_map_lookup_is_exact(
    source: &str,
    value: &str,
    statement_start: usize,
    variable: &str,
) -> Option<bool> {
    let value = lua_trim_start_comments(value)?;
    let Some(identifier) = lua_identifier_literal_from_query(value) else {
        return Some(false);
    };
    if identifier != variable {
        return Some(false);
    }
    let rest = lua_trim_start_comments(value.get(identifier.len()..)?)?;
    let Some(after_open) = rest.strip_prefix('[').and_then(lua_trim_start_comments) else {
        return Some(false);
    };
    if lua_quoted_string_literal_from_query(after_open)
        .or_else(|| lua_long_bracket_literal_from_query(after_open))
        .is_none()
    {
        let Some(key_variable) = lua_identifier_literal_from_query(after_open) else {
            return Some(false);
        };
        if !lua_static_builtin_scheme_string_key_is_stable_for_lookup(
            source,
            key_variable,
            statement_start,
        )? {
            return Some(false);
        }
    }
    let Some((name, tail)) =
        lua_static_bracket_string_key_from_query(source, statement_start, rest)
    else {
        return Some(false);
    };
    if builtin_color_scheme_toml(&name).is_none() {
        return Some(false);
    }
    lua_static_builtin_scheme_tail_is_statement_end(tail)
}

fn lua_static_builtin_scheme_string_key_is_stable_for_lookup(
    source: &str,
    variable: &str,
    lookup_statement_start: usize,
) -> Option<bool> {
    let (_, binding_start) =
        lua_static_builtin_scheme_binding_before_offset(source, variable, lookup_statement_start)?;
    let starts = lua_top_level_statement_start_indices_before_offset(source, source.len())?;
    let lookup_statement_index = starts
        .iter()
        .rposition(|start| *start <= lookup_statement_start)?;
    let mut capturing_functions = Vec::new();

    for (index, start) in starts
        .iter()
        .copied()
        .enumerate()
        .take(lookup_statement_index)
    {
        let end = starts
            .get(index + 1)
            .copied()
            .unwrap_or(lookup_statement_start)
            .min(lookup_statement_start);
        let statement = source.get(start..end)?;
        if let Some(function) = lua_static_builtin_scheme_function_definition_name(statement)? {
            if lua_static_query_contains_identifier(statement, variable)? {
                capturing_functions.push(function);
            }
            continue;
        }
        if start <= binding_start {
            continue;
        }
        if lua_static_query_contains_identifier(statement, variable)? {
            return Some(false);
        }
        for captured in &capturing_functions {
            if lua_static_query_contains_identifier(statement, captured)? {
                return Some(false);
            }
        }
    }

    let lookup_statement_end = starts
        .get(lookup_statement_index + 1)
        .copied()
        .unwrap_or(source.len());
    let lookup_statement = source.get(lookup_statement_start..lookup_statement_end)?;
    for captured in &capturing_functions {
        if lua_static_query_contains_identifier(lookup_statement, captured)? {
            return Some(false);
        }
    }
    Some(true)
}

fn lua_static_builtin_scheme_tail_is_statement_end(tail: &str) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(tail)?;
    let tail = normalized.trim();
    Some(
        tail.is_empty()
            || tail
                .strip_prefix(';')
                .is_some_and(|tail| tail.trim().is_empty()),
    )
}

fn lua_static_builtin_scheme_fragment_references_map_identity(
    fragment: &str,
    variable: &str,
    capturing_functions: &[String],
) -> Option<bool> {
    if lua_static_query_contains_identifier(fragment, variable)? {
        return Some(true);
    }
    for captured in capturing_functions {
        if lua_static_query_contains_identifier(fragment, captured)? {
            return Some(true);
        }
    }
    Some(false)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_static_builtin_scheme_map_rebind_is_safe(
    source: &str,
    statement: &str,
    statement_start: usize,
    variable: &str,
    capturing_functions: &[String],
) -> Option<Option<bool>> {
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
    let statement = normalized.trim_start();
    let statement = if lua_source_keyword_at(statement, 0, "local") {
        statement.get("local".len()..)?.trim_start()
    } else {
        statement
    };
    let Some((targets, value)) = split_lua_static_load_scheme_path_assignment_statement(statement)
    else {
        return Some(None);
    };
    let targets = split_lua_top_level_arguments(targets)?;
    let [target] = targets.as_slice() else {
        return Some(None);
    };
    if lua_static_load_scheme_path_assignment_target_identifier(target).as_deref() != Some(variable)
    {
        return Some(None);
    }
    if lua_static_builtin_scheme_fragment_references_map_identity(
        value,
        variable,
        capturing_functions,
    )? {
        return Some(Some(false));
    }
    Some(Some(lua_static_builtin_scheme_rebind_value_is_fresh(
        source,
        value,
        statement_start,
    )?))
}

fn lua_static_builtin_scheme_rebind_value_is_fresh(
    source: &str,
    value: &str,
    statement_start: usize,
) -> Option<bool> {
    if lua_static_builtin_scheme_fresh_map_call_is_exact(source, value, statement_start)? {
        return Some(true);
    }
    lua_static_builtin_scheme_independent_table_literal_is_exact(value, 0)
}

fn lua_static_builtin_scheme_fresh_map_call_is_exact(
    source: &str,
    value: &str,
    statement_start: usize,
) -> Option<bool> {
    let canonical = lua_static_wezterm_builtin_color_scheme_call_query_from_query(
        source,
        value,
        statement_start,
    )
    .or_else(|| {
        lua_static_wezterm_builtin_color_scheme_alias_query_from_query(
            source,
            value,
            statement_start,
        )
    });
    let Some(canonical) = canonical else {
        return Some(false);
    };
    let rest = canonical.strip_prefix("wezterm.color.get_builtin_schemes")?;
    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let (arguments, tail) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_static_load_scheme_path_query_without_comments(arguments)?
        .trim()
        .is_empty()
    {
        return Some(false);
    }
    lua_static_builtin_scheme_tail_is_statement_end(tail)
}

fn lua_static_builtin_scheme_independent_table_literal_is_exact(
    value: &str,
    depth: usize,
) -> Option<bool> {
    if depth >= LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return Some(false);
    }
    let value = lua_trim_start_comments(value)?;
    let Some(table) = lua_braced_table_literal_from_query(value) else {
        return Some(false);
    };
    if !lua_static_builtin_scheme_tail_is_statement_end(value.get(table.len()..)?)? {
        return Some(false);
    }
    let fields = table.trim().strip_prefix('{')?.strip_suffix('}')?;
    for field in split_lua_table_top_level_fields(fields)? {
        let field = lua_trim_start_comments(field)?.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, field_value)) = split_lua_table_assignment_from_field(field) {
            if !lua_static_builtin_scheme_static_table_key_is_exact(key)?
                || !lua_static_builtin_scheme_static_value_is_exact(field_value, depth + 1)?
            {
                return Some(false);
            }
        } else if !lua_static_builtin_scheme_static_value_is_exact(field, depth + 1)? {
            return Some(false);
        }
    }
    Some(true)
}

fn lua_static_builtin_scheme_static_table_key_is_exact(key: &str) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(key)?;
    let key = normalized.trim();
    if let Some(identifier) = lua_identifier_literal_from_query(key) {
        return Some(identifier.len() == key.len());
    }
    let Some(key) = key.strip_prefix('[').and_then(|key| key.strip_suffix(']')) else {
        return Some(false);
    };
    lua_static_builtin_scheme_static_scalar_is_exact(key)
}

fn lua_static_builtin_scheme_static_value_is_exact(value: &str, depth: usize) -> Option<bool> {
    let value = lua_trim_start_comments(value)?;
    if value.starts_with('{') {
        return lua_static_builtin_scheme_independent_table_literal_is_exact(value, depth);
    }
    lua_static_builtin_scheme_static_scalar_is_exact(value)
}

fn lua_static_builtin_scheme_static_scalar_is_exact(value: &str) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(value)?;
    let value = normalized.trim();
    if value == "nil" {
        return Some(true);
    }
    let literal = lua_quoted_string_literal_from_query(value)
        .or_else(|| lua_long_bracket_literal_from_query(value))
        .or_else(|| lua_signed_number_literal_from_query(value))
        .or_else(|| lua_bool_literal_from_query(value));
    let Some(literal) = literal else {
        return Some(false);
    };
    Some(literal.len() == value.len())
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
fn lua_static_builtin_scheme_function_definition_name(statement: &str) -> Option<Option<String>> {
    let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
    let statement = normalized.trim_start();
    if lua_source_keyword_at(statement, 0, "function") {
        let rest = statement.get("function".len()..)?.trim_start();
        let Some(name) = lua_identifier_literal_from_query(rest) else {
            return Some(None);
        };
        return Some(
            rest.get(name.len()..)?
                .trim_start()
                .starts_with('(')
                .then(|| name.to_owned()),
        );
    }
    let statement = if lua_source_keyword_at(statement, 0, "local") {
        let rest = statement.get("local".len()..)?.trim_start();
        if lua_source_keyword_at(rest, 0, "function") {
            let rest = rest.get("function".len()..)?.trim_start();
            let Some(name) = lua_identifier_literal_from_query(rest) else {
                return Some(None);
            };
            return Some(
                rest.get(name.len()..)?
                    .trim_start()
                    .starts_with('(')
                    .then(|| name.to_owned()),
            );
        }
        rest
    } else {
        statement
    };
    let Some((targets, value)) = split_lua_static_load_scheme_path_assignment_statement(statement)
    else {
        return Some(None);
    };
    if !lua_source_keyword_at(value.trim_start(), 0, "function") {
        return Some(None);
    }
    let targets = split_lua_top_level_arguments(targets)?;
    let [target] = targets.as_slice() else {
        return Some(None);
    };
    Some(lua_static_load_scheme_path_assignment_target_identifier(
        target,
    ))
}

fn lua_static_query_contains_identifier(query: &str, variable: &str) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(query)?;
    let mut quote = None;
    let mut escape = false;
    let mut long_bracket_end = None;

    for (index, character) in normalized.char_indices() {
        if let Some(end) = long_bracket_end {
            if index < end {
                continue;
            }
            long_bracket_end = None;
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
        if matches!(character, '\'' | '"') {
            quote = Some(character);
            continue;
        }
        if character == '['
            && let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(normalized.get(index..)?)
        {
            let content_and_rest = normalized.get(index + content_start..)?;
            long_bracket_end = Some(
                content_and_rest
                    .find(&closing)
                    .map_or(normalized.len(), |close_index| {
                        index + content_start + close_index + closing.len()
                    }),
            );
            continue;
        }
        if !normalized.get(index..)?.starts_with(variable) {
            continue;
        }
        let previous = normalized
            .get(..index)?
            .chars()
            .rev()
            .find(|character| !character.is_whitespace());
        let next = normalized.get(index + variable.len()..)?.chars().next();
        if !previous.is_some_and(|character| {
            is_lua_identifier_character(character) || matches!(character, '.' | ':')
        }) && !next.is_some_and(is_lua_identifier_character)
        {
            return Some(true);
        }
    }

    Some(false)
}

fn lua_config_colors_variable_source_before_offset<'a>(
    source: &'a str,
    variable: &str,
    reference_start: usize,
) -> Option<NativeConfigColorsLuaSource<'a>> {
    match lua_color_variable_source_before_offset(source, variable, reference_start)? {
        NativeColorSchemeLuaSource::Table {
            colors, variable, ..
        } => Some(NativeConfigColorsLuaSource::Table { colors, variable }),
        NativeColorSchemeLuaSource::LoadScheme { path, variable, .. } => {
            Some(NativeConfigColorsLuaSource::LoadScheme(
                NativeLoadSchemeColorsAssignment { path, variable },
            ))
        }
        NativeColorSchemeLuaSource::Builtin { name, variable, .. } => {
            Some(NativeConfigColorsLuaSource::Builtin(
                NativeBuiltinColorSchemeAssignment { name, variable },
            ))
        }
        NativeColorSchemeLuaSource::DefaultColors { variable, .. } => {
            Some(NativeConfigColorsLuaSource::DefaultColors { variable })
        }
    }
}

fn lua_builtin_color_scheme_assignment_from_query(
    source: &str,
    query: &str,
    variable: &str,
) -> Option<String> {
    let value = lua_color_variable_whole_assignment_value_from_query(query, variable)?;
    lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(source, value)
        .or_else(|| lua_whole_map_builtin_color_scheme_name_from_query(source, value))
}

fn lua_config_load_scheme_colors_assignment_from_query(
    source: &str,
) -> Option<NativeLoadSchemeColorsAssignment> {
    lua_config_assignment_from_query(source, "colors", Some)
        .and_then(|value| {
            lua_wezterm_color_load_scheme_path_from_query_with_static_source(source, value)
        })
        .map(|path| NativeLoadSchemeColorsAssignment {
            path,
            variable: None,
        })
        .or_else(|| {
            let colors_variable = lua_config_assignment_from_query(
                source,
                "colors",
                lua_identifier_literal_from_query,
            )?;
            let reference_start = lua_source_slice_start_offset(source, colors_variable)?;
            let NativeColorSchemeLuaSource::LoadScheme { path, variable, .. } =
                lua_color_variable_source_before_offset(source, colors_variable, reference_start)?
            else {
                return None;
            };
            Some(NativeLoadSchemeColorsAssignment { path, variable })
        })
}

fn lua_load_scheme_assignment_path_from_query(
    source: &str,
    query: &str,
    variable: &str,
) -> Option<String> {
    let rest = if let Some(rest) = query.trim_start().strip_prefix("local") {
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            return None;
        }
        lua_trim_start_comments(rest)?
    } else {
        query.trim_start()
    };
    let (names, value) = rest.split_once('=')?;
    if names.contains('\n') || names.contains('\r') || names.contains(';') {
        return None;
    }
    let first_name = names.split(',').next()?.trim();
    if first_name != variable {
        return None;
    }
    lua_wezterm_color_load_scheme_path_from_query_with_static_source(source, value)
}

fn lua_color_variable_mutation_table_from_query(
    source: &str,
    variable: &str,
    statement_start: usize,
    statement_end: usize,
) -> Option<String> {
    let mut fields = Vec::new();
    let mut indexed_fields = BTreeMap::new();
    let statement = source.get(statement_start..statement_end)?;
    let rest = statement.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = statement.get(variable.len()..)?.trim_start();
    let (field_name, rest) = lua_color_variable_mutation_field_from_query_with_static_key(
        source,
        rest,
        statement_start,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    if field_name == "tab_bar"
        || field_name == "indexed"
            && lua_color_variable_mutation_array_index_from_query(rest).is_some()
    {
        return None;
    }
    let value = lua_color_variable_mutation_value_literal_from_query(rest.strip_prefix('=')?)?;
    if value.is_empty() {
        return None;
    }
    if matches!(field_name.as_str(), "ansi" | "brights") {
        let value = value.trim();
        if value.starts_with('{')
            && value
                .strip_prefix('{')?
                .strip_suffix('}')?
                .trim()
                .is_empty()
        {
            return None;
        }
    }
    if lua_color_spec_field_name(&field_name) {
        let value = value.trim();
        if value.starts_with('{')
            && value
                .strip_prefix('{')?
                .strip_suffix('}')?
                .trim()
                .is_empty()
        {
            return None;
        }
    }
    if field_name == "indexed" {
        let indexed_table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        for entry in split_lua_table_top_level_fields(indexed_table)? {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let (index, color) = split_lua_table_assignment_from_field(entry)?;
            let index = split_lua_table_array_index_from_query(index.trim())?;
            indexed_fields.insert(index, color.trim().to_owned());
        }
    } else {
        fields.push(format!("{field_name} = {value}"));
    }

    if !indexed_fields.is_empty() {
        let indexed_fields = indexed_fields
            .into_iter()
            .map(|(index, value)| format!("[{index}] = {value}"))
            .collect::<Vec<_>>()
            .join(",\n");
        fields.push(format!("indexed = {{\n{indexed_fields}\n}}"));
    }

    (!fields.is_empty()).then(|| format!("{{\n{}\n}}", fields.join(",\n")))
}

fn apply_lua_color_variable_indexed_palette_slot_mutation_overrides(
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
    if field_name != "indexed" {
        return Some(false);
    }
    let rest = lua_trim_start_comments(rest)?;
    let Some((index, rest)) = lua_color_variable_mutation_array_index_from_query(rest) else {
        return Some(false);
    };
    if !(16..=255).contains(&index) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let Some(value) = rest.strip_prefix('=') else {
        return Some(false);
    };
    let value = parse_maybe_quoted_query_text(
        lua_color_variable_mutation_value_literal_from_query(value)?,
    )?;
    let mut palette = overrides.indexed_palette.unwrap_or([None; 256]);
    palette[index] = Some(lua_opaque_color_from_query_with_static_source(
        Some(LuaStaticSource {
            source,
            max_start: statement_start,
        }),
        &value,
    )?);
    overrides.indexed_palette = Some(palette);

    Some(true)
}

fn apply_lua_color_variable_palette_slot_mutation_overrides(
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
    let Some((field_name, rest)) = lua_color_variable_mutation_field_from_query(rest) else {
        return Some(false);
    };
    let Some(offset) = (match field_name.as_str() {
        "ansi" => Some(0),
        "brights" => Some(8),
        _ => None,
    }) else {
        return Some(false);
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
            return Some(false);
        };
        let value = parse_maybe_quoted_query_text(
            lua_color_variable_mutation_value_literal_from_query(value)?,
        )?;
        palette[offset + index - 1] = lua_opaque_color_from_query_with_static_source(
            Some(LuaStaticSource {
                source,
                max_start: statement_start,
            }),
            &value,
        )?;
    } else {
        let Some(value) = rest.strip_prefix('=') else {
            return Some(false);
        };
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        let values = split_lua_table_string_array_with_static_source(
            Some(LuaStaticSource {
                source,
                max_start: statement_start,
            }),
            value,
        )?;
        let colors = values
            .iter()
            .map(|value| {
                lua_opaque_color_from_query_with_static_source(
                    Some(LuaStaticSource {
                        source,
                        max_start: statement_start,
                    }),
                    value,
                )
            })
            .collect::<Option<Vec<_>>>()?;
        let colors = <[Color; 8]>::try_from(colors).ok()?;
        palette[offset..offset + 8].copy_from_slice(&colors);
    }
    overrides.ansi_palette = Some(palette);

    Some(true)
}

fn apply_lua_color_variable_tab_bar_mutation_overrides(
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
    let Some((field_name, rest)) = lua_color_variable_mutation_field_from_query(rest) else {
        return Some(false);
    };
    if field_name != "tab_bar" {
        return Some(false);
    }

    apply_lua_tab_bar_color_mutation_rest(source, rest, statement_start, overrides)
}

fn apply_lua_tab_bar_color_mutation_rest(
    source: &str,
    rest: &str,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let rest = lua_trim_start_comments(rest)?;
    if let Some(value) = rest.strip_prefix('=') {
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        return apply_lua_colors_table_overrides(
            Some(LuaStaticSource { source, max_start }),
            &format!("{{\ntab_bar = {value}\n}}"),
            overrides,
        );
    }

    let Some((tab_bar_field, rest)) =
        lua_color_variable_mutation_field_from_query_with_static_key(source, rest, max_start)
    else {
        return Some(false);
    };
    let rest = lua_trim_start_comments(rest)?;
    if tab_bar_field == "background" {
        let Some(value) = rest.strip_prefix('=') else {
            return Some(false);
        };
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        let value = parse_maybe_quoted_query_text(value)?;
        overrides.tab_bar_background_color = Some(lua_opaque_color_from_query_with_static_source(
            Some(LuaStaticSource { source, max_start }),
            &value,
        )?);
        return Some(true);
    }
    if tab_bar_field == "inactive_tab_edge" {
        let Some(value) = rest.strip_prefix('=') else {
            return Some(false);
        };
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        let value = parse_maybe_quoted_query_text(value)?;
        overrides.tab_bar_inactive_tab_edge_color =
            Some(lua_opaque_color_from_query_with_static_source(
                Some(LuaStaticSource { source, max_start }),
                &value,
            )?);
        return Some(true);
    }

    if !lua_tab_bar_item_color_name(&tab_bar_field) {
        return Some(false);
    }
    if let Some(value) = rest.strip_prefix('=') {
        let value = lua_color_variable_mutation_value_literal_from_query(value)?;
        return apply_lua_colors_table_overrides(
            Some(LuaStaticSource { source, max_start }),
            &format!("{{\ntab_bar = {{ {tab_bar_field} = {value} }}\n}}"),
            overrides,
        );
    }

    let Some((item_field, rest)) =
        lua_color_variable_mutation_field_from_query_with_static_key(source, rest, max_start)
    else {
        return Some(false);
    };
    let rest = lua_trim_start_comments(rest)?;
    let Some(value) = rest.strip_prefix('=') else {
        return Some(false);
    };
    let value = lua_color_variable_mutation_value_literal_from_query(value)?;
    apply_lua_tab_bar_item_color_mutation(
        Some(LuaStaticSource { source, max_start }),
        overrides,
        &tab_bar_field,
        &item_field,
        value,
    )
}

fn lua_tab_bar_item_color_name(value: &str) -> bool {
    matches!(
        value,
        "active_tab" | "inactive_tab" | "inactive_tab_hover" | "new_tab" | "new_tab_hover"
    )
}

fn apply_lua_tab_bar_item_color_mutation(
    static_source: Option<LuaStaticSource<'_>>,
    overrides: &mut NativeConfigSnapshot,
    item_name: &str,
    field_name: &str,
    value: &str,
) -> Option<bool> {
    let colors = match item_name {
        "active_tab" => &mut overrides.tab_bar_active_tab_colors,
        "inactive_tab" => &mut overrides.tab_bar_inactive_tab_colors,
        "inactive_tab_hover" => &mut overrides.tab_bar_inactive_tab_hover_colors,
        "new_tab" => &mut overrides.tab_bar_new_tab_colors,
        "new_tab_hover" => &mut overrides.tab_bar_new_tab_hover_colors,
        _ => return Some(false),
    };

    match field_name {
        "fg_color" => {
            let value = parse_maybe_quoted_query_text(value)?;
            colors.fg_color = Some(lua_opaque_color_from_query_with_static_source(
                static_source,
                &value,
            )?);
        }
        "bg_color" => {
            let value = parse_maybe_quoted_query_text(value)?;
            colors.bg_color = Some(lua_opaque_color_from_query_with_static_source(
                static_source,
                &value,
            )?);
        }
        "intensity" => {
            let value = parse_maybe_quoted_query_text(value)?;
            colors.intensity = Some(tab_bar_item_intensity_from_query(&value)?);
        }
        "underline" => {
            let value = parse_maybe_quoted_query_text(value)?;
            colors.underline = Some(tab_bar_item_underline_from_query(&value)?);
        }
        "italic" => colors.italic = Some(lua_bool_literal_from_query(value)? == "true"),
        "strikethrough" => {
            colors.strikethrough = Some(lua_bool_literal_from_query(value)? == "true");
        }
        _ => return Some(false),
    }

    Some(true)
}

fn lua_color_variable_mutation_array_index_from_query(query: &str) -> Option<(usize, &str)> {
    let query = lua_trim_start_comments(query)?;
    let rest = query.strip_prefix('[')?;
    let (index, rest) = rest.split_once(']')?;
    let index = index.trim().parse::<usize>().ok()?;

    Some((index, rest))
}

fn lua_color_variable_mutation_field_from_query_with_static_key<'a>(
    source: &str,
    query: &'a str,
    max_start: usize,
) -> Option<(String, &'a str)> {
    let query = lua_trim_start_comments(query)?;
    if let Some(rest) = query.strip_prefix('.') {
        let field_name = lua_identifier_literal_from_query(rest)?;
        return Some((field_name.to_owned(), rest.get(field_name.len()..)?));
    }

    let rest = query.strip_prefix('[')?;
    let rest = lua_trim_start_comments(rest)?;
    if let Some(literal) = lua_quoted_string_literal_from_query(rest)
        .or_else(|| lua_long_bracket_literal_from_query(rest))
    {
        let field_name = parse_maybe_quoted_query_text(literal)?;
        let after_literal = lua_trim_start_comments(rest.get(literal.len()..)?)?;
        let after_bracket = after_literal.strip_prefix(']')?;

        return Some((
            non_empty_spawn_command_option_value(&field_name).ok()?,
            after_bracket,
        ));
    }

    let variable = lua_identifier_literal_from_query(rest)?;
    let field_name =
        lua_static_string_variable_assignment_before_offset_from_query(source, variable, max_start)
            .and_then(parse_maybe_quoted_query_text)?;
    let after_variable = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let after_bracket = after_variable.strip_prefix(']')?;

    Some((
        non_empty_spawn_command_option_value(&field_name).ok()?,
        after_bracket,
    ))
}

fn lua_color_variable_mutation_field_from_query(query: &str) -> Option<(String, &str)> {
    let query = lua_trim_start_comments(query)?;
    if let Some(rest) = query.strip_prefix('.') {
        let field_name = lua_identifier_literal_from_query(rest)?;
        return Some((field_name.to_owned(), rest.get(field_name.len()..)?));
    }

    let rest = query.strip_prefix('[')?;
    let rest = lua_trim_start_comments(rest)?;
    let literal = lua_quoted_string_literal_from_query(rest)
        .or_else(|| lua_long_bracket_literal_from_query(rest))?;
    let field_name = parse_maybe_quoted_query_text(literal)?;
    let after_literal = lua_trim_start_comments(rest.get(literal.len()..)?)?;
    let after_bracket = after_literal.strip_prefix(']')?;

    Some((
        non_empty_spawn_command_option_value(&field_name).ok()?,
        after_bracket,
    ))
}

fn lua_color_variable_mutation_value_literal_from_query(query: &str) -> Option<&str> {
    let query = lua_trim_start_comments(query)?;
    lua_braced_table_literal_from_query(query)
        .or_else(|| lua_quoted_string_literal_from_query(query))
        .or_else(|| lua_long_bracket_literal_from_query(query))
        .or_else(|| {
            let value = query
                .split_once('\n')
                .map_or(query, |(value, _)| value)
                .trim()
                .trim_end_matches(',')
                .trim();
            (!value.is_empty()).then_some(value)
        })
}

const LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH: usize = 8;

#[allow(dead_code)]
fn lua_static_load_scheme_path_query_without_comments(value: &str) -> Option<String> {
    let mut normalized = String::with_capacity(value.len());
    let mut index = 0usize;
    let mut quote = None;
    let mut escape = false;

    while index < value.len() {
        let character = value.get(index..)?.chars().next()?;
        if let Some(active_quote) = quote {
            normalized.push(character);
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == active_quote {
                quote = None;
            }
            index += character.len_utf8();
            continue;
        }

        if value[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&value[index + 2..])
            {
                let content_and_rest = value.get(index + 2 + content_start..)?;
                let close_index = content_and_rest.find(&closing)?;
                let end = index + 2 + content_start + close_index + closing.len();
                for character in value.get(index..end)?.chars() {
                    if character == '\n' {
                        normalized.push(character);
                    } else {
                        for _ in 0..character.len_utf8() {
                            normalized.push(' ');
                        }
                    }
                }
                index = end;
                continue;
            }

            let rest = value.get(index + 2..)?;
            let end = if let Some(newline) = rest.find('\n') {
                index + 2 + newline + '\n'.len_utf8()
            } else {
                value.len()
            };
            for character in value.get(index..end)?.chars() {
                if character == '\n' {
                    normalized.push(character);
                } else {
                    for _ in 0..character.len_utf8() {
                        normalized.push(' ');
                    }
                }
            }
            index = end;
            continue;
        }

        if matches!(character, '\'' | '"') {
            quote = Some(character);
            normalized.push(character);
            index += character.len_utf8();
            continue;
        }

        if character == '['
            && let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(value.get(index..)?)
        {
            let content_and_rest = value.get(index + content_start..)?;
            let close_index = content_and_rest.find(&closing)?;
            let end = index + content_start + close_index + closing.len();
            normalized.push_str(value.get(index..end)?);
            index = end;
            continue;
        }

        normalized.push(character);
        index += character.len_utf8();
    }

    (quote.is_none() && normalized.len() == value.len()).then_some(normalized)
}

fn lua_static_load_scheme_path_expression_value_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    lua_static_load_scheme_path_expression_value_from_query_with_depth(source, query, max_start, 0)
}

#[allow(dead_code)]
fn lua_static_load_scheme_path_expression_value_from_query_with_depth(
    source: &str,
    query: &str,
    max_start: usize,
    depth: usize,
) -> Option<String> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }

    let query = lua_static_load_scheme_path_query_without_comments(query)?;
    let query = query.trim();
    if let Some((value, literal_len)) = lua_inline_string_literal_value_and_len(query)
        && query.get(literal_len..)?.trim().is_empty()
    {
        return Some(value);
    }

    if let Some(rest) = lua_static_wezterm_receiver_rest_from_query(source, max_start, query) {
        let static_source = Some(LuaStaticSource { source, max_start });
        let rest = lua_trim_start_comments(rest)?;
        let (field, rest) =
            lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
        if field == "config_dir" && lua_static_value_tail_is_value_end(rest) {
            return std::env::var("WEZTERM_CONFIG_DIR").ok();
        }
    }

    if query == "wezterm.config_dir" {
        return std::env::var("WEZTERM_CONFIG_DIR").ok();
    }

    if let Some(segments) = split_lua_string_concat_segments(query) {
        let mut value = String::new();
        for segment in segments {
            value.push_str(
                &lua_static_load_scheme_path_expression_value_from_query_with_depth(
                    source,
                    segment,
                    max_start,
                    depth + 1,
                )?,
            );
        }
        return Some(value);
    }

    let variable = lua_identifier_literal_from_query(query)?;
    if variable.len() != query.len() {
        return None;
    }
    let (value, assignment_start) =
        lua_static_load_scheme_path_binding_before_offset(source, variable, max_start)?;
    lua_static_load_scheme_path_expression_value_from_query_with_depth(
        source,
        value,
        assignment_start,
        depth + 1,
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
#[allow(dead_code)]
fn split_lua_static_load_scheme_path_assignment_statement(statement: &str) -> Option<(&str, &str)> {
    let mut table_depth = 0u32;
    let mut paren_depth = 0u32;
    let mut bracket_depth = 0u32;
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut assignment = None;

    for (index, character) in statement.char_indices() {
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

        if statement[index..].starts_with("--") {
            let block_comment = if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&statement[index + 2..])
            {
                let content_and_rest = &statement[index + 2 + content_start..];
                let close_index = content_and_rest.find(&closing)?;
                Some(index + 2 + content_start + close_index + closing.len())
            } else {
                None
            };
            let top_level = table_depth == 0 && paren_depth == 0 && bracket_depth == 0;
            if top_level && let Some((lhs_end, rhs_start)) = assignment {
                let rhs_before_comment = statement.get(rhs_start..index)?;
                if rhs_before_comment.trim().is_empty() {
                    if let Some(end) = block_comment {
                        assignment = Some((lhs_end, end));
                        block_comment_end = Some(end);
                    } else {
                        line_comment = true;
                    }
                    continue;
                }
                let remainder = lua_trim_start_comments(statement.get(index..)?)?;
                let trailing_comment = if remainder.trim().is_empty() {
                    true
                } else if let Some(after_semicolon) = remainder.trim_start().strip_prefix(';') {
                    lua_trim_start_comments(after_semicolon)?.trim().is_empty()
                } else {
                    false
                };
                if !trailing_comment {
                    if let Some(end) = block_comment {
                        block_comment_end = Some(end);
                    } else {
                        line_comment = true;
                    }
                    continue;
                }
                return Some((
                    statement.get(..lhs_end)?,
                    lua_trim_start_comments(rhs_before_comment)?,
                ));
            }
            if let Some(end) = block_comment {
                block_comment_end = Some(end);
            } else {
                line_comment = true;
            }
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            '[' => {
                let query = statement.get(index..)?;
                if !lua_bracket_starts_complete_long_string_index(query)
                    && let Some((content_start, closing)) = parse_lua_long_bracket_delimiters(query)
                {
                    let content_and_rest = &statement[index + content_start..];
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
            ';' if table_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                if let Some((lhs_end, rhs_start)) = assignment {
                    return Some((
                        statement.get(..lhs_end)?,
                        lua_trim_start_comments(statement.get(rhs_start..index)?)?,
                    ));
                }
            }
            '=' if table_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                if assignment.is_none() {
                    let previous = statement.get(..index)?.chars().next_back();
                    let rest = statement.get(index + character.len_utf8()..)?;
                    if !matches!(previous, Some('=' | '~' | '<' | '>')) && !rest.starts_with('=') {
                        assignment = Some((index, index + character.len_utf8()));
                    }
                }
            }
            _ => {}
        }
    }

    let (lhs_end, rhs_start) = assignment?;
    Some((
        statement.get(..lhs_end)?,
        lua_trim_start_comments(statement.get(rhs_start..)?)?,
    ))
}

#[allow(dead_code)]
fn lua_static_load_scheme_path_statement_starts_with_continuation(statement: &str) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
    let statement = normalized.trim_start();
    let symbol_continuation = [
        "..", "==", "~=", "<=", ">=", "//", "<<", ">>", "+", "-", "*", "/", "%", "^", "&", "|",
        "~", "<", ">", "=", ".", ",", "[",
    ]
    .iter()
    .any(|operator| statement.starts_with(operator));
    Some(
        symbol_continuation
            || (statement.starts_with(':') && !statement.starts_with("::"))
            || lua_source_keyword_at(statement, 0, "and")
            || lua_source_keyword_at(statement, 0, "or"),
    )
}

#[allow(dead_code)]
fn lua_static_load_scheme_path_statement_ends_with_continuation(statement: &str) -> Option<bool> {
    let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
    let statement = normalized.trim_end();
    let statement_without_labels =
        lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
    if statement_without_labels.ends_with('>')
        && lua_source_keyword_at(statement_without_labels.trim_start(), 0, "local")
    {
        let declaration = lua_trim_start_comments(
            statement_without_labels
                .trim_start()
                .get("local".len()..)
                .unwrap_or_default(),
        )?;
        let targets = split_lua_top_level_arguments(declaration)?;
        if targets.iter().all(|target| {
            lua_static_load_scheme_path_assignment_target_identifier(target).is_some()
        }) {
            return Some(false);
        }
    }
    let symbol_continuation = [
        "..", "==", "~=", "<=", ">=", "//", "<<", ">>", "+", "-", "*", "/", "%", "^", "&", "|",
        "~", "<", ">", "=", ".", ",",
    ]
    .iter()
    .any(|operator| statement.ends_with(operator));
    let ends_with_keyword = |keyword: &str| {
        statement.strip_suffix(keyword).is_some_and(|prefix| {
            !prefix
                .chars()
                .next_back()
                .is_some_and(is_lua_identifier_character)
        })
    };
    Some(
        symbol_continuation
            || (statement.ends_with(':') && !statement.ends_with("::"))
            || ends_with_keyword("and")
            || ends_with_keyword("or")
            || ends_with_keyword("local"),
    )
}

#[allow(dead_code)]
fn lua_static_load_scheme_path_statements_continue_across_boundary(
    before: &str,
    after: &str,
) -> Option<bool> {
    let normalized_before = lua_static_load_scheme_path_query_without_comments(before)?;
    if normalized_before.trim_end().ends_with(';') {
        return Some(false);
    }
    Some(
        lua_static_load_scheme_path_statement_ends_with_continuation(before)?
            || lua_static_load_scheme_path_statement_starts_with_continuation(after)?,
    )
}

fn lua_static_load_scheme_path_assignment_target_identifier(target: &str) -> Option<String> {
    let target = lua_static_load_scheme_path_query_without_comments(target)?;
    let target = target.trim();
    let identifier = lua_identifier_literal_from_query(target)?;

    let rest = target.get(identifier.len()..)?;
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(identifier.to_owned());
    }

    let rest = lua_trim_start_comments(rest.strip_prefix('<')?)?;
    let attribute = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(attribute.len()..).unwrap_or_default())?;
    rest.strip_prefix('>')
        .filter(|rest| rest.trim().is_empty())
        .map(|_| identifier.to_owned())
}

fn lua_static_load_scheme_path_assignment_target_is_variable(target: &str, variable: &str) -> bool {
    lua_static_load_scheme_path_assignment_target_identifier(target).as_deref() == Some(variable)
}

enum LuaStaticLoadSchemePathBinding<'a> {
    Unbound,
    Value(&'a str, usize),
    Shadowed,
}

fn lua_static_load_scheme_path_statement_without_leading_labels(statement: &str) -> Option<&str> {
    let mut statement = lua_trim_start_comments(statement)?;
    loop {
        let Some(rest) = statement.strip_prefix("::") else {
            return Some(statement);
        };
        let rest = lua_trim_start_comments(rest)?;
        let label = lua_identifier_literal_from_query(rest)?;
        let rest = lua_trim_start_comments(rest.get(label.len()..)?)?;
        let rest = rest.strip_prefix("::")?;
        statement = lua_trim_start_comments(rest)?;
    }
}

#[allow(dead_code)]
fn lua_static_load_scheme_path_binding_before_offset<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
) -> Option<(&'a str, usize)> {
    match lua_static_load_scheme_path_binding_state_before_offset(source, variable, max_start)? {
        LuaStaticLoadSchemePathBinding::Value(value, start) => Some((value, start)),
        LuaStaticLoadSchemePathBinding::Unbound | LuaStaticLoadSchemePathBinding::Shadowed => None,
    }
}

fn lua_static_load_scheme_path_binding_state_before_offset<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
) -> Option<LuaStaticLoadSchemePathBinding<'a>> {
    let mut selected = LuaStaticLoadSchemePathBinding::Unbound;
    let normalized_source =
        lua_static_load_scheme_path_query_without_comments(source.get(..max_start)?)?;
    let starts =
        lua_top_level_statement_start_indices_before_offset(&normalized_source, max_start)?;
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

        let statement_end = starts
            .get(next_statement_index)
            .copied()
            .unwrap_or(max_start);
        let statement = source.get(start..statement_end)?;
        let statement = lua_static_load_scheme_path_statement_without_leading_labels(statement)?;
        statement_index = next_statement_index;
        if lua_source_keyword_at(statement, 0, "return") {
            continue;
        }
        if lua_source_keyword_at(statement, 0, "function") {
            if lua_named_function_params_and_body_from_statement(statement, variable).is_some() {
                selected = LuaStaticLoadSchemePathBinding::Shadowed;
            }
            continue;
        }
        let is_local = lua_source_keyword_at(statement, 0, "local");
        let rest = if is_local {
            statement.get("local".len()..)?
        } else {
            statement
        };
        let rest = lua_trim_start_comments(rest)?;
        if is_local && lua_source_keyword_at(rest, 0, "function") {
            let function_rest = lua_trim_start_comments(rest.get("function".len()..)?)?;
            if lua_identifier_literal_from_query(function_rest) == Some(variable) {
                selected = LuaStaticLoadSchemePathBinding::Shadowed;
            }
            continue;
        }
        let assignment = split_lua_static_load_scheme_path_assignment_statement(rest);
        let targets =
            split_lua_top_level_arguments(assignment.map_or(rest, |(targets, _)| targets))?;
        let mut declares_variable = false;
        for target in &targets {
            if lua_static_load_scheme_path_assignment_target_is_variable(target, variable) {
                declares_variable = true;
                break;
            }
        }
        if !declares_variable {
            continue;
        }

        if let Some((_, value)) = assignment {
            selected = if targets.len() == 1 {
                LuaStaticLoadSchemePathBinding::Value(value, start)
            } else {
                LuaStaticLoadSchemePathBinding::Shadowed
            };
        } else if is_local {
            selected = LuaStaticLoadSchemePathBinding::Shadowed;
        }
    }

    Some(selected)
}

fn lua_wezterm_color_load_scheme_path_from_call_query(
    source: &str,
    canonical_query: &str,
    call_max_start: usize,
) -> Option<String> {
    let rest = lua_function_name_rest_from_query(
        canonical_query.trim_start(),
        "wezterm.color.load_scheme",
    )?;
    let rest = lua_trim_start_comments(rest)?;

    if let Some(arguments) = rest.strip_prefix('(') {
        let (arguments, tail) = lua_parenthesized_argument_list_prefix_from_query(arguments)?;
        let arguments = split_lua_top_level_arguments(arguments)?;
        let [argument] = arguments.as_slice() else {
            return None;
        };
        if lua_static_load_scheme_path_query_without_comments(argument)?
            .trim()
            .is_empty()
            || !lua_static_value_tail_is_value_end(tail)
        {
            return None;
        }
        return lua_static_load_scheme_path_expression_value_from_query(
            source,
            argument,
            call_max_start,
        );
    }

    let (literal, literal_len) = lua_inline_string_literal_value_and_len(rest)?;
    lua_static_value_tail_is_value_end(rest.get(literal_len..)?).then_some(literal)
}

fn lua_static_value_tail_is_value_end(mut tail: &str) -> bool {
    let mut saw_statement_boundary = false;

    loop {
        let mut whitespace_end = 0usize;
        for (index, character) in tail.char_indices() {
            if !character.is_whitespace() {
                break;
            }
            if character == '\n' {
                saw_statement_boundary = true;
            }
            whitespace_end = index + character.len_utf8();
        }
        tail = match tail.get(whitespace_end..) {
            Some(tail) => tail,
            None => return false,
        };

        let Some(comment) = tail.strip_prefix("--") else {
            break;
        };
        saw_statement_boundary = true;
        if let Some((content_start, closing)) = parse_lua_long_bracket_delimiters(comment) {
            let Some(content_and_rest) = comment.get(content_start..) else {
                return false;
            };
            let Some(close_index) = content_and_rest.find(&closing) else {
                return false;
            };
            let Some(rest) = content_and_rest.get(close_index + closing.len()..) else {
                return false;
            };
            tail = rest;
            continue;
        }

        let Some(newline) = comment.find('\n') else {
            return true;
        };
        let Some(rest) = comment.get(newline + '\n'.len_utf8()..) else {
            return false;
        };
        tail = rest;
    }

    if tail.is_empty()
        || tail.starts_with(';')
        || tail.starts_with(',')
        || tail.starts_with('}')
        || lua_static_value_tail_starts_label_statement(tail)
    {
        return true;
    }
    if lua_static_value_tail_starts_expression_continuation(tail) {
        return false;
    }

    saw_statement_boundary && lua_identifier_literal_from_query(tail).is_some()
}

fn lua_static_value_tail_starts_label_statement(tail: &str) -> bool {
    let Some(rest) = tail.strip_prefix("::") else {
        return false;
    };
    let Some(label) = lua_identifier_literal_from_query(rest) else {
        return false;
    };
    rest.get(..label.len()) == Some(label)
        && rest
            .get(label.len()..)
            .is_some_and(|rest| rest.starts_with("::"))
}

fn lua_static_value_tail_starts_expression_continuation(tail: &str) -> bool {
    matches!(
        tail.chars().next(),
        Some(
            '.' | '['
                | '('
                | ':'
                | '\''
                | '"'
                | '{'
                | '+'
                | '-'
                | '*'
                | '/'
                | '%'
                | '^'
                | '&'
                | '|'
                | '~'
                | '<'
                | '>'
                | '='
        )
    ) || lua_source_keyword_at(tail, 0, "and")
        || lua_source_keyword_at(tail, 0, "or")
}

fn lua_wezterm_color_load_scheme_path_from_query_with_static_source(
    source: &str,
    query: &str,
) -> Option<String> {
    let call_max_start = lua_source_slice_start_offset(source, query)?;
    if let Some(path) =
        lua_wezterm_color_load_scheme_path_from_call_query(source, query, call_max_start)
    {
        return Some(path);
    }

    if let Some(canonical_query) =
        lua_static_wezterm_color_load_scheme_call_query_from_query(source, query, call_max_start)
    {
        return lua_wezterm_color_load_scheme_path_from_call_query(
            source,
            &canonical_query,
            call_max_start,
        );
    }

    let canonical_query =
        lua_static_wezterm_color_load_scheme_alias_query_from_query(source, query, call_max_start)?;
    lua_wezterm_color_load_scheme_path_from_call_query(source, &canonical_query, call_max_start)
}

fn lua_static_wezterm_color_load_scheme_call_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = lua_trim_start_comments(query)?;
    let rest = lua_static_wezterm_receiver_rest_from_query(source, max_start, query)?;
    let rest = lua_trim_start_comments(rest)?;
    let static_source = Some(LuaStaticSource { source, max_start });
    let (field, rest) = lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
    if field != "color" {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
    if field != "load_scheme" {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    if !matches!(rest.chars().next()?, '(' | '\'' | '"' | '[') {
        return None;
    }
    Some(format!("wezterm.color.load_scheme{rest}"))
}

fn lua_static_wezterm_color_load_scheme_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = query.trim_start();
    let alias = lua_identifier_literal_from_query(query)?;
    if !lua_static_wezterm_color_load_scheme_alias_before_offset(source, alias, max_start)? {
        return None;
    }

    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    if !matches!(rest.chars().next()?, '(' | '\'' | '"' | '[') {
        return None;
    }

    Some(format!("wezterm.color.load_scheme{rest}"))
}

fn lua_static_wezterm_color_load_scheme_alias_before_offset(
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
        selected =
            lua_static_wezterm_color_load_scheme_alias_value_from_query(source, start, value);
    }

    Some(selected)
}

fn lua_static_wezterm_color_load_scheme_alias_value_from_query(
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
    field == "load_scheme" && lua_static_identifier_value_rest_is_statement_end(rest)
}

#[allow(dead_code)]
fn lua_wezterm_builtin_color_scheme_name_from_query_with_static_source(
    source: &str,
    query: &str,
) -> Option<String> {
    let lookup_max_start = lua_source_slice_start_offset(source, query)?;
    if lua_static_builtin_scheme_wezterm_receiver_is_valid(source, lookup_max_start)?
        && let Some(name) =
            lua_wezterm_builtin_color_scheme_name_from_call_query(source, query, lookup_max_start)
    {
        return Some(name);
    }

    if let Some(canonical_query) = lua_static_wezterm_builtin_color_scheme_call_query_from_query(
        source,
        query,
        lookup_max_start,
    ) {
        return lua_wezterm_builtin_color_scheme_name_from_call_query(
            source,
            &canonical_query,
            lookup_max_start,
        );
    }

    let canonical_query = lua_static_wezterm_builtin_color_scheme_alias_query_from_query(
        source,
        query,
        lookup_max_start,
    )?;
    lua_wezterm_builtin_color_scheme_name_from_call_query(
        source,
        &canonical_query,
        lookup_max_start,
    )
}

#[allow(dead_code)]
fn lua_wezterm_builtin_color_scheme_name_from_call_query(
    source: &str,
    canonical_query: &str,
    lookup_max_start: usize,
) -> Option<String> {
    let query = canonical_query.trim_start();
    let rest = query.strip_prefix("wezterm.color.get_builtin_schemes")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }

    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = lua_static_load_scheme_path_query_without_comments(arguments)?;
    if !arguments.trim().is_empty() {
        return None;
    }

    let rest = lua_trim_start_comments(rest)?;
    let (name, tail) = lua_static_bracket_string_key_from_query(source, lookup_max_start, rest)?;
    builtin_color_scheme_toml(&name)?;
    lua_static_value_tail_is_value_end(tail).then_some(name)
}

#[allow(dead_code)]
fn lua_static_bracket_string_key_from_query<'a>(
    source: &str,
    max_start: usize,
    query: &'a str,
) -> Option<(String, &'a str)> {
    let after_open = lua_trim_start_comments(query.strip_prefix('[')?)?;
    if let Some((key, key_len)) = lua_inline_string_literal_value_and_len(after_open) {
        let rest = lua_trim_start_comments(after_open.get(key_len..)?)?;
        return Some((key, rest.strip_prefix(']')?));
    }

    let variable = lua_identifier_literal_from_query(after_open)?;
    let rest = lua_trim_start_comments(after_open.get(variable.len()..)?)?;
    let rest = rest.strip_prefix(']')?;
    let key = lua_static_string_literal_binding_before_offset(source, variable, max_start)?;
    Some((key, rest))
}

#[allow(dead_code)]
fn lua_static_builtin_scheme_binding_before_offset<'a>(
    source: &'a str,
    variable: &str,
    max_start: usize,
) -> Option<(&'a str, usize)> {
    let selected = lua_static_load_scheme_path_binding_before_offset(source, variable, max_start)?;
    if lua_static_builtin_scheme_has_non_function_block_between(source, selected.1, max_start)? {
        return None;
    }
    Some(selected)
}

#[allow(dead_code)]
fn lua_static_builtin_scheme_has_non_function_block_between(
    source: &str,
    min_start: usize,
    max_start: usize,
) -> Option<bool> {
    let source = source.get(min_start..max_start)?;
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut blocks = Vec::new();

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

        if matches!(character, '\'' | '"') {
            quote = Some(character);
            continue;
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

        if lua_source_keyword_at(source, index, "elseif") {
            if blocks.last() == Some(&false) {
                blocks.pop();
            }
            continue;
        }
        if lua_source_keyword_at(source, index, "function") {
            blocks.push(true);
            continue;
        }
        if lua_source_keyword_at(source, index, "then")
            || lua_source_keyword_at(source, index, "do")
            || lua_source_keyword_at(source, index, "repeat")
        {
            if !blocks.contains(&true) {
                // Dynamic branches are outside this static resolver. Even an
                // unrelated block makes bindings selected before it unprovable.
                return Some(true);
            }
            blocks.push(false);
            continue;
        }
        if lua_source_keyword_at(source, index, "end")
            || lua_source_keyword_at(source, index, "until")
        {
            blocks.pop();
        }
    }

    Some(false)
}

#[allow(dead_code)]
fn lua_static_builtin_scheme_require_is_unshadowed(source: &str, max_start: usize) -> Option<bool> {
    match lua_static_load_scheme_path_binding_state_before_offset(source, "require", max_start)? {
        LuaStaticLoadSchemePathBinding::Unbound => Some(
            !lua_static_builtin_scheme_has_non_function_block_between(source, 0, max_start)?,
        ),
        LuaStaticLoadSchemePathBinding::Value(_, _) | LuaStaticLoadSchemePathBinding::Shadowed => {
            Some(false)
        }
    }
}

#[allow(dead_code)]
fn lua_static_builtin_scheme_wezterm_receiver_is_valid(
    source: &str,
    max_start: usize,
) -> Option<bool> {
    lua_static_builtin_scheme_wezterm_receiver_is_valid_with_depth(source, max_start, 0)
}

fn lua_static_builtin_scheme_wezterm_receiver_is_valid_with_depth(
    source: &str,
    max_start: usize,
    depth: usize,
) -> Option<bool> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return Some(false);
    }
    match lua_static_load_scheme_path_binding_state_before_offset(source, "wezterm", max_start)? {
        LuaStaticLoadSchemePathBinding::Unbound => Some(
            !lua_static_builtin_scheme_has_non_function_block_between(source, 0, max_start)?,
        ),
        LuaStaticLoadSchemePathBinding::Value(binding, binding_start) => {
            if lua_static_builtin_scheme_has_non_function_block_between(
                source,
                binding_start,
                max_start,
            )? {
                return Some(false);
            }
            Some(lua_static_wezterm_module_alias_value_is_exact_with_depth(
                source,
                binding_start,
                binding,
                depth + 1,
            ))
        }
        LuaStaticLoadSchemePathBinding::Shadowed => Some(false),
    }
}

#[allow(dead_code)]
fn lua_static_string_literal_binding_before_offset(
    source: &str,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    let (value, _) = lua_static_builtin_scheme_binding_before_offset(source, variable, max_start)?;
    let value = lua_static_load_scheme_path_query_without_comments(value)?;
    let value = value.trim();
    let (literal, literal_len) = lua_inline_string_literal_value_and_len(value)?;
    value
        .get(literal_len..)?
        .trim()
        .is_empty()
        .then_some(literal)
}

#[allow(dead_code)]
fn lua_static_string_field_key_from_query<'a>(
    source: &str,
    max_start: usize,
    query: &'a str,
) -> Option<(String, &'a str)> {
    if let Some(rest) = query.strip_prefix('.') {
        let rest = lua_trim_start_comments(rest)?;
        let field = lua_identifier_literal_from_query(rest)?;
        return Some((field.to_owned(), rest.get(field.len()..)?));
    }

    lua_static_bracket_string_key_from_query(source, max_start, query)
}

#[allow(dead_code)]
fn lua_static_wezterm_module_alias_value_is_exact(
    source: &str,
    max_start: usize,
    value: &str,
) -> bool {
    lua_static_wezterm_module_alias_value_is_exact_with_depth(source, max_start, value, 0)
}

#[allow(dead_code)]
fn lua_static_wezterm_module_alias_value_is_exact_with_depth(
    source: &str,
    max_start: usize,
    value: &str,
    depth: usize,
) -> bool {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return false;
    }
    let Some(value) = lua_trim_start_comments(value) else {
        return false;
    };

    if let Some(rest) = value.strip_prefix('(')
        && let Some((receiver, tail)) = lua_parenthesized_argument_list_prefix_from_query(rest)
        && lua_static_wezterm_module_alias_value_is_exact_with_depth(
            source,
            max_start,
            receiver.trim(),
            depth + 1,
        )
        && lua_static_value_tail_is_value_end(tail)
    {
        return true;
    }

    if let Some(rest) = lua_static_wezterm_require_receiver_rest_from_query(value) {
        return lua_static_builtin_scheme_require_is_unshadowed(source, max_start).unwrap_or(false)
            && lua_static_value_tail_is_value_end(rest);
    }

    let Some(rest) = value.strip_prefix("wezterm") else {
        return false;
    };
    !rest.chars().next().is_some_and(is_lua_identifier_character)
        && lua_static_builtin_scheme_wezterm_receiver_is_valid_with_depth(
            source,
            max_start,
            depth + 1,
        )
        .unwrap_or(false)
        && lua_static_value_tail_is_value_end(rest)
}

#[allow(dead_code)]
fn lua_static_wezterm_receiver_rest_from_query_with_strict_aliases<'a>(
    source: &str,
    max_start: usize,
    value: &'a str,
) -> Option<&'a str> {
    lua_static_wezterm_receiver_rest_from_query_with_strict_aliases_and_depth(
        source, max_start, value, 0,
    )
}

fn lua_static_wezterm_receiver_rest_from_query_with_strict_aliases_and_depth<'a>(
    source: &str,
    max_start: usize,
    value: &'a str,
    depth: usize,
) -> Option<&'a str> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }
    if let Some(value) = lua_trim_start_comments(value)
        && let Some(rest) = value.strip_prefix('(')
        && let Some((receiver, rest)) = lua_parenthesized_argument_list_prefix_from_query(rest)
    {
        let receiver_rest =
            lua_static_wezterm_receiver_rest_from_query_with_strict_aliases_and_depth(
                source,
                max_start,
                receiver.trim(),
                depth + 1,
            )?;
        if lua_static_wezterm_module_alias_receiver_rest_is_statement_end(receiver_rest) {
            return Some(rest);
        }
    }

    if let Some(rest) = lua_static_wezterm_require_receiver_rest_from_query(value) {
        return lua_static_builtin_scheme_require_is_unshadowed(source, max_start)?.then_some(rest);
    }

    let receiver = lua_identifier_literal_from_query(value)?;
    let rest = value.get(receiver.len()..)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    if receiver == "wezterm" {
        return lua_static_builtin_scheme_wezterm_receiver_is_valid_with_depth(
            source,
            max_start,
            depth + 1,
        )?
        .then_some(rest);
    }

    let (binding, binding_start) =
        lua_static_builtin_scheme_binding_before_offset(source, receiver, max_start)?;
    lua_static_wezterm_module_alias_value_is_exact_with_depth(
        source,
        binding_start,
        binding,
        depth + 1,
    )
    .then_some(rest)
}

#[allow(dead_code)]
fn lua_static_wezterm_builtin_color_scheme_function_rest_from_receiver_rest<'a>(
    source: &str,
    max_start: usize,
    receiver_rest: &'a str,
) -> Option<&'a str> {
    let rest = lua_trim_start_comments(receiver_rest)?;
    let (field, rest) = lua_static_string_field_key_from_query(source, max_start, rest)?;
    if field == "get_builtin_color_schemes" {
        return Some(rest);
    }
    if field != "color" {
        return None;
    }

    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_static_string_field_key_from_query(source, max_start, rest)?;
    (field == "get_builtin_schemes").then_some(rest)
}

#[allow(dead_code)]
fn lua_static_wezterm_builtin_color_scheme_call_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = lua_trim_start_comments(query)?;
    let receiver_rest =
        lua_static_wezterm_receiver_rest_from_query_with_strict_aliases(source, max_start, query)?;
    let rest = lua_static_wezterm_builtin_color_scheme_function_rest_from_receiver_rest(
        source,
        max_start,
        receiver_rest,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with('(') {
        return None;
    }
    Some(format!("wezterm.color.get_builtin_schemes{rest}"))
}

#[allow(dead_code)]
fn lua_static_wezterm_builtin_color_scheme_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = lua_trim_start_comments(query)?;
    let alias = lua_identifier_literal_from_query(query)?;
    if !lua_static_wezterm_builtin_color_scheme_alias_before_offset(source, alias, max_start) {
        return None;
    }

    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    if !rest.starts_with('(') {
        return None;
    }
    Some(format!("wezterm.color.get_builtin_schemes{rest}"))
}

#[allow(dead_code)]
fn lua_static_wezterm_builtin_color_scheme_alias_before_offset(
    source: &str,
    alias: &str,
    max_start: usize,
) -> bool {
    let Some((value, binding_start)) =
        lua_static_builtin_scheme_binding_before_offset(source, alias, max_start)
    else {
        return false;
    };
    lua_static_wezterm_builtin_color_scheme_alias_value_from_query(source, binding_start, value)
}

#[allow(dead_code)]
fn lua_static_wezterm_builtin_color_scheme_alias_value_from_query(
    source: &str,
    max_start: usize,
    value: &str,
) -> bool {
    let Some(value) = lua_trim_start_comments(value) else {
        return false;
    };
    let receiver_rest = if let Some(rest) =
        lua_static_wezterm_require_receiver_rest_from_query(value)
    {
        if !lua_static_builtin_scheme_require_is_unshadowed(source, max_start).unwrap_or(false) {
            return false;
        }
        rest
    } else if let Some(rest) =
        lua_static_wezterm_receiver_rest_from_query_with_strict_aliases(source, max_start, value)
    {
        rest
    } else {
        return false;
    };
    let Some(rest) = lua_static_wezterm_builtin_color_scheme_function_rest_from_receiver_rest(
        source,
        max_start,
        receiver_rest,
    ) else {
        return false;
    };
    lua_static_value_tail_is_value_end(rest)
}

#[allow(dead_code)]
fn lua_wezterm_default_colors_from_query_with_static_source(
    source: &str,
    query: &str,
) -> Option<()> {
    let call_max_start = lua_source_slice_start_offset(source, query)?;
    if !lua_static_wezterm_default_colors_api_is_unmodified_before_offset(source, call_max_start)? {
        return None;
    }
    let canonical_query =
        lua_static_wezterm_default_colors_call_query_from_query(source, query, call_max_start)
            .or_else(|| {
                lua_static_wezterm_default_colors_alias_query_from_query(
                    source,
                    query,
                    call_max_start,
                )
            })?;
    lua_wezterm_default_colors_from_call_query(&canonical_query)
}

fn lua_static_wezterm_default_colors_api_is_unmodified_before_offset(
    source: &str,
    max_start: usize,
) -> Option<bool> {
    let statements = lua_top_level_logical_statements_before_offset(source, max_start)?;
    for statement_range in statements {
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
                if lua_static_wezterm_default_colors_assignment_target_may_modify_api(
                    source,
                    target,
                    statement_range.start,
                )? {
                    return Some(false);
                }
            }
            continue;
        }

        if !lua_source_keyword_at(statement, 0, "function") {
            continue;
        }
        let normalized = lua_static_load_scheme_path_query_without_comments(statement)?;
        let function_rest = normalized.get("function".len()..)?.trim_start();
        let Some((target, _)) = function_rest.split_once('(') else {
            continue;
        };
        let target = target.replace(':', ".");
        if lua_static_wezterm_default_colors_assignment_target_may_modify_api(
            source,
            &target,
            statement_range.start,
        )? {
            return Some(false);
        }
    }

    Some(true)
}

fn lua_static_wezterm_default_colors_assignment_target_may_modify_api(
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
        if rest.is_empty() {
            return Some(false);
        }
        let Some((field, rest)) = lua_static_string_field_key_from_query(source, max_start, rest)
        else {
            return Some(rest.starts_with('.') || rest.starts_with('['));
        };
        if field != "color" {
            return Some(false);
        }
        return lua_static_wezterm_default_colors_namespace_target_rest_may_modify_api(
            source, rest, max_start,
        );
    }

    let Some(rest) =
        lua_static_wezterm_color_namespace_rest_from_query_with_depth(source, target, max_start, 0)
    else {
        return Some(false);
    };
    lua_static_wezterm_default_colors_namespace_target_rest_may_modify_api(source, rest, max_start)
}

fn lua_static_wezterm_default_colors_namespace_target_rest_may_modify_api(
    source: &str,
    rest: &str,
    max_start: usize,
) -> Option<bool> {
    let rest = lua_trim_start_comments(rest)?;
    if rest.is_empty() {
        return Some(true);
    }
    let Some((field, tail)) = lua_static_string_field_key_from_query(source, max_start, rest)
    else {
        return Some(rest.starts_with('.') || rest.starts_with('['));
    };
    if field != "get_default_colors" {
        return Some(false);
    }
    Some(
        lua_static_load_scheme_path_query_without_comments(tail)?
            .trim()
            .is_empty(),
    )
}

fn lua_wezterm_default_colors_from_call_query(canonical_query: &str) -> Option<()> {
    let query = canonical_query.trim_start();
    let rest = query.strip_prefix("wezterm.color.get_default_colors")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }

    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let (arguments, tail) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_static_load_scheme_path_query_without_comments(arguments)?
        .trim()
        .is_empty()
        || !lua_static_value_tail_is_value_end(tail)
    {
        return None;
    }
    Some(())
}

fn lua_static_wezterm_default_colors_call_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let rest = lua_static_wezterm_default_colors_function_rest_from_query_with_depth(
        source, query, max_start, 0,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    rest.starts_with('(')
        .then(|| format!("wezterm.color.get_default_colors{rest}"))
}

fn lua_static_wezterm_default_colors_alias_query_from_query(
    source: &str,
    query: &str,
    max_start: usize,
) -> Option<String> {
    let query = lua_trim_start_comments(query)?;
    let alias = lua_identifier_literal_from_query(query)?;
    let (value, binding_start) =
        lua_static_builtin_scheme_binding_before_offset(source, alias, max_start)?;
    if !lua_static_wezterm_default_colors_function_value_is_exact_with_depth(
        source,
        value,
        binding_start,
        0,
    ) {
        return None;
    }

    let rest = lua_trim_start_comments(query.get(alias.len()..)?)?;
    rest.starts_with('(')
        .then(|| format!("wezterm.color.get_default_colors{rest}"))
}

fn lua_static_wezterm_default_colors_function_rest_from_query_with_depth<'a>(
    source: &str,
    query: &'a str,
    max_start: usize,
    depth: usize,
) -> Option<&'a str> {
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
    (field == "get_default_colors").then_some(rest)
}

fn lua_static_wezterm_default_colors_function_value_is_exact_with_depth(
    source: &str,
    value: &str,
    max_start: usize,
    depth: usize,
) -> bool {
    let Some(rest) = lua_static_wezterm_default_colors_function_rest_from_query_with_depth(
        source, value, max_start, depth,
    ) else {
        return false;
    };
    lua_static_value_tail_is_value_end(rest)
}

fn lua_static_wezterm_module_namespace_rest_from_query_with_depth<'a>(
    source: &str,
    value: &'a str,
    max_start: usize,
    depth: usize,
) -> Option<&'a str> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }
    let value = lua_trim_start_comments(value)?;

    if let Some(rest) = value.strip_prefix('(')
        && let Some((module, tail)) = lua_parenthesized_argument_list_prefix_from_query(rest)
        && let Some(module_rest) = lua_static_wezterm_module_namespace_rest_from_query_with_depth(
            source,
            module.trim(),
            max_start,
            depth + 1,
        )
        && lua_static_value_tail_is_value_end(module_rest)
    {
        return Some(tail);
    }

    if let Some(rest) = lua_static_wezterm_receiver_rest_from_query_with_strict_aliases_and_depth(
        source,
        max_start,
        value,
        depth + 1,
    ) {
        return Some(rest);
    }

    let alias = lua_identifier_literal_from_query(value)?;
    let rest = value.get(alias.len()..)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let (binding, binding_start) =
        lua_static_builtin_scheme_binding_before_offset(source, alias, max_start)?;
    lua_static_wezterm_module_namespace_value_is_exact_with_depth(
        source,
        binding,
        binding_start,
        depth + 1,
    )
    .then_some(rest)
}

fn lua_static_wezterm_module_namespace_value_is_exact_with_depth(
    source: &str,
    value: &str,
    max_start: usize,
    depth: usize,
) -> bool {
    let Some(rest) = lua_static_wezterm_module_namespace_rest_from_query_with_depth(
        source, value, max_start, depth,
    ) else {
        return false;
    };
    lua_static_value_tail_is_value_end(rest)
}

fn lua_static_wezterm_color_namespace_rest_from_query_with_depth<'a>(
    source: &str,
    value: &'a str,
    max_start: usize,
    depth: usize,
) -> Option<&'a str> {
    if depth > LUA_STATIC_LOAD_SCHEME_PATH_MAX_DEPTH {
        return None;
    }
    let value = lua_trim_start_comments(value)?;

    if let Some(rest) = value.strip_prefix('(')
        && let Some((namespace, tail)) = lua_parenthesized_argument_list_prefix_from_query(rest)
        && let Some(namespace_rest) = lua_static_wezterm_color_namespace_rest_from_query_with_depth(
            source,
            namespace.trim(),
            max_start,
            depth + 1,
        )
        && lua_static_value_tail_is_value_end(namespace_rest)
    {
        return Some(tail);
    }

    if let Some(receiver_rest) = lua_static_wezterm_module_namespace_rest_from_query_with_depth(
        source,
        value,
        max_start,
        depth + 1,
    ) && let Some(rest) = lua_trim_start_comments(receiver_rest)
        && let Some((field, rest)) = lua_static_string_field_key_from_query(source, max_start, rest)
        && field == "color"
    {
        return Some(rest);
    }

    let alias = lua_identifier_literal_from_query(value)?;
    let rest = value.get(alias.len()..)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let (binding, binding_start) =
        lua_static_builtin_scheme_binding_before_offset(source, alias, max_start)?;
    lua_static_wezterm_color_namespace_value_is_exact_with_depth(
        source,
        binding,
        binding_start,
        depth + 1,
    )
    .then_some(rest)
}

fn lua_static_wezterm_color_namespace_value_is_exact_with_depth(
    source: &str,
    value: &str,
    max_start: usize,
    depth: usize,
) -> bool {
    let Some(rest) = lua_static_wezterm_color_namespace_rest_from_query_with_depth(
        source, value, max_start, depth,
    ) else {
        return false;
    };
    lua_static_value_tail_is_value_end(rest)
}

fn lua_identifier_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    let mut chars = query.char_indices();
    let (_, first) = chars.next()?;
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    let mut end = first.len_utf8();
    for (index, character) in chars {
        if !is_lua_identifier_character(character) {
            break;
        }
        end = index + character.len_utf8();
    }
    query.get(..end)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
#[allow(dead_code)]
fn lua_config_assignment_from_query<'a>(
    source: &'a str,
    field: &str,
    mut literal_from_query: impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
    if let Some(table) = lua_config_static_return_table_from_query(source) {
        let max_start = lua_source_slice_start_offset(source, table)?;
        return lua_config_table_field_assignment_from_query_with_static_source(
            Some(LuaStaticSource { source, max_start }),
            table,
            field,
            &mut literal_from_query,
        );
    }
    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");

    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut selected = None;

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
                    if let Some(table) = lua_braced_table_literal_from_query(after_assignment) {
                        let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
                        if let Some(value) =
                            lua_config_table_field_assignment_from_query_with_static_source(
                                Some(LuaStaticSource {
                                    source,
                                    max_start: index,
                                }),
                                table,
                                field,
                                &mut literal_from_query,
                            )
                        {
                            selected = Some(value);
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
                && let Some(value) = literal_from_query(lua_trim_start_comments(rest)?)
            {
                selected = Some(value);
            }
        }

        if character == '['
            && lua_block_depth == 0
            && let Some(rest) =
                lua_config_bracket_assignment_rest_from_query(source, index, receiver, field)
            && let Some(rest) = lua_trim_start_comments(rest)?.strip_prefix('=')
            && let Some(value) = literal_from_query(lua_trim_start_comments(rest)?)
        {
            selected = Some(value);
        }
    }

    selected.or_else(|| {
        lua_config_table_initializer_assignment_from_query(
            source,
            receiver,
            field,
            &mut literal_from_query,
        )
    })
}

#[allow(dead_code)]
fn lua_config_dot_assignment_has_config_receiver(source: &str, start: usize) -> bool {
    lua_config_dot_assignment_has_receiver(source, start, "config")
}

fn lua_config_dot_assignment_has_receiver(source: &str, start: usize, receiver: &str) -> bool {
    let prefix = source[..start].trim_end();
    let Some(receiver_prefix) = prefix.strip_suffix('.') else {
        return false;
    };
    let receiver_prefix = receiver_prefix.trim_end();
    let Some(receiver_start) = receiver_prefix.len().checked_sub(receiver.len()) else {
        return false;
    };
    if &receiver_prefix[receiver_start..] != receiver {
        return false;
    }
    let before_receiver = receiver_prefix[..receiver_start].chars().next_back();
    !before_receiver.is_some_and(is_lua_identifier_character)
}

#[allow(dead_code)]
fn lua_config_return_table_assignment_from_query<'a>(
    source: &'a str,
    field: &str,
    literal_from_query: &mut impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
    let table = lua_config_static_return_table_from_query(source)?;
    lua_config_table_field_assignment_from_query(table, field, literal_from_query)
}

fn lua_config_table_field_assignment_from_query<'a>(
    table: &'a str,
    field: &str,
    literal_from_query: &mut impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
    lua_config_table_field_assignment_from_query_with_static_source(
        None,
        table,
        field,
        literal_from_query,
    )
}

fn lua_config_table_field_assignment_from_query_with_static_source<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    table: &'a str,
    field: &str,
    literal_from_query: &mut impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
    let mut selected = None;

    for table_field in split_lua_table_top_level_fields(table)? {
        let Some((key, value)) = split_lua_table_assignment_from_field(table_field.trim()) else {
            continue;
        };
        let Some(key) =
            split_lua_table_key_from_query_with_static_source(static_source, key.trim())
        else {
            continue;
        };
        if key == field {
            selected = literal_from_query(lua_trim_start_comments(value)?);
        }
    }

    selected
}

fn lua_config_table_field_assignment_string_from_query<'a>(
    table: &'a str,
    field: &str,
    literal_from_query: &mut impl FnMut(&'a str) -> Option<String>,
) -> Option<String> {
    lua_config_table_field_assignment_string_from_query_with_static_source(
        None,
        table,
        field,
        literal_from_query,
    )
}

fn lua_config_table_field_assignment_string_from_query_with_static_source<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    table: &'a str,
    field: &str,
    literal_from_query: &mut impl FnMut(&'a str) -> Option<String>,
) -> Option<String> {
    let mut selected = None;

    for table_field in split_lua_table_top_level_fields(table)? {
        let Some((key, value)) = split_lua_table_assignment_from_field(table_field.trim()) else {
            continue;
        };
        let Some(key) =
            split_lua_table_key_from_query_with_static_source(static_source, key.trim())
        else {
            continue;
        };
        if key == field {
            selected = literal_from_query(lua_trim_start_comments(value)?);
        }
    }

    selected
}

fn lua_config_table_value_field_assignment_from_table_query(
    source: &str,
    table: &str,
    field: &str,
    max_start: usize,
) -> Option<LuaTableValueAssignment> {
    let mut selected = None;
    let static_source = Some(LuaStaticSource { source, max_start });

    for table_field in split_lua_table_top_level_fields(table)? {
        let Some((key, value)) = split_lua_table_assignment_from_field(table_field.trim()) else {
            continue;
        };
        let Some(key) =
            split_lua_table_key_from_query_with_static_source(static_source, key.trim())
        else {
            continue;
        };
        if key == field {
            selected = lua_table_insert_value_table_assignment_from_query(
                source,
                lua_trim_start_comments(value)?,
                max_start,
            );
        }
    }

    selected
}

fn lua_config_string_array_field_assignment_from_table_query(
    source: &str,
    table: &str,
    field: &str,
    max_start: usize,
) -> Option<LuaTableValueAssignment> {
    let mut selected = None;
    let static_source = Some(LuaStaticSource { source, max_start });

    for table_field in split_lua_table_top_level_fields(table)? {
        let Some((key, value)) = split_lua_table_assignment_from_field(table_field.trim()) else {
            continue;
        };
        let Some(key) =
            split_lua_table_key_from_query_with_static_source(static_source, key.trim())
        else {
            continue;
        };
        if key == field {
            selected = lua_string_array_value_table_assignment_from_query(
                source,
                lua_trim_start_comments(value)?,
                max_start,
            );
        }
    }

    selected
}

fn lua_config_u32_array_field_assignment_from_table_query(
    source: &str,
    table: &str,
    field: &str,
    max_start: usize,
) -> Option<LuaTableValueAssignment> {
    let mut selected = None;
    let static_source = Some(LuaStaticSource { source, max_start });

    for table_field in split_lua_table_top_level_fields(table)? {
        let Some((key, value)) = split_lua_table_assignment_from_field(table_field.trim()) else {
            continue;
        };
        let Some(key) =
            split_lua_table_key_from_query_with_static_source(static_source, key.trim())
        else {
            continue;
        };
        if key == field {
            selected = lua_u32_array_value_table_assignment_from_query(
                source,
                lua_trim_start_comments(value)?,
                max_start,
            );
        }
    }

    selected
}

fn lua_config_table_map_field_assignment_from_table_query(
    source: &str,
    table: &str,
    field: &str,
    max_start: usize,
) -> Option<LuaTableMapAssignment> {
    let mut selected = None;
    let static_source = Some(LuaStaticSource { source, max_start });

    for table_field in split_lua_table_top_level_fields(table)? {
        let Some((key, value)) = split_lua_table_assignment_from_field(table_field.trim()) else {
            continue;
        };
        let Some(key) =
            split_lua_table_key_from_query_with_static_source(static_source, key.trim())
        else {
            continue;
        };
        if key == field {
            selected = lua_table_map_assignment_from_query(
                source,
                lua_trim_start_comments(value)?,
                max_start,
            );
        }
    }

    selected
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_config_static_return_table_from_query(source: &str) -> Option<&str> {
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;

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

        if source[index..].starts_with("return")
            && lua_config_assignment_field_has_boundaries(source, index, "return")
            && lua_block_depth == 0
        {
            let rest = lua_trim_start_comments(source.get(index + "return".len()..)?)?;
            let Some(table) = lua_braced_table_literal_from_query(rest) else {
                continue;
            };
            return table
                .trim()
                .strip_prefix('{')?
                .strip_suffix('}')
                .map(str::trim);
        }
    }

    None
}

fn lua_config_static_return_identifier_from_query(source: &str) -> Option<&str> {
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;

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

        if source[index..].starts_with("return")
            && lua_config_assignment_field_has_boundaries(source, index, "return")
            && lua_block_depth == 0
        {
            let rest = lua_trim_start_comments(source.get(index + "return".len()..)?)?;
            let identifier = lua_identifier_literal_from_query(rest)?;
            let after_identifier = lua_trim_start_comments(rest.get(identifier.len()..)?)?;
            if after_identifier.is_empty() || after_identifier.starts_with(',') {
                return Some(identifier);
            }
        }
    }

    None
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
#[allow(dead_code)]
fn lua_config_table_initializer_assignment_from_query<'a>(
    source: &'a str,
    receiver: &str,
    field: &str,
    literal_from_query: &mut impl FnMut(&'a str) -> Option<&'a str>,
) -> Option<&'a str> {
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;

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
                let Some(after_config) = lua_config_receiver_prefix_rest(rest, receiver) else {
                    continue;
                };
                after_config
            } else if let Some(after_config) =
                lua_config_receiver_prefix_rest(source.get(index..)?, receiver)
            {
                after_config
            } else {
                continue;
            };

            let after_config = lua_trim_start_comments(after_config)?;
            let Some(after_assignment) = after_config.strip_prefix('=') else {
                continue;
            };
            let after_assignment = lua_trim_start_comments(after_assignment)?;
            let Some(table) = lua_braced_table_literal_from_query(after_assignment) else {
                continue;
            };
            let static_source = Some(LuaStaticSource {
                source,
                max_start: index,
            });
            let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
            for table_field in split_lua_table_top_level_fields(table)? {
                let Some((key, value)) = split_lua_table_assignment_from_field(table_field.trim())
                else {
                    continue;
                };
                let Some(key) =
                    split_lua_table_key_from_query_with_static_source(static_source, key.trim())
                else {
                    continue;
                };
                if key == field {
                    return literal_from_query(lua_trim_start_comments(value)?);
                }
            }
        }
    }

    None
}

fn lua_config_receiver_prefix_rest<'a>(query: &'a str, receiver: &str) -> Option<&'a str> {
    let after_receiver = query.strip_prefix(receiver)?;
    if after_receiver
        .chars()
        .next()
        .is_some_and(is_lua_identifier_character)
    {
        return None;
    }
    Some(after_receiver)
}

#[allow(dead_code)]
fn lua_source_keyword_at(source: &str, index: usize, keyword: &str) -> bool {
    source[index..].starts_with(keyword)
        && lua_config_assignment_field_has_boundaries(source, index, keyword)
}

#[allow(dead_code)]
fn lua_trim_start_comments(mut source: &str) -> Option<&str> {
    loop {
        let trimmed = source.trim_start();
        let rest = trimmed.strip_prefix("--");
        let Some(rest) = rest else {
            return Some(trimmed);
        };

        if let Some((content_start, closing)) = parse_lua_long_bracket_delimiters(rest) {
            let content_and_rest = &rest[content_start..];
            let close_index = content_and_rest.find(&closing)?;
            source = &content_and_rest[close_index + closing.len()..];
            continue;
        }

        let newline = rest.find('\n')?;
        source = &rest[newline + '\n'.len_utf8()..];
    }
}

fn lua_trim_end_comments(mut source: &str) -> Option<&str> {
    loop {
        let trimmed = source.trim_end();
        let mut quote = None;
        let mut escape = false;
        let mut line_comment = false;
        let mut block_comment_end = None;
        let mut long_bracket_end = None;
        let mut brace_depth = 0u32;
        let mut paren_depth = 0u32;
        let mut bracket_depth = 0u32;
        let mut removed_comment = false;

        for (index, character) in trimmed.char_indices() {
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

            if trimmed[index..].starts_with("--") {
                let top_level = brace_depth == 0 && paren_depth == 0 && bracket_depth == 0;
                if let Some((content_start, closing)) =
                    parse_lua_long_bracket_delimiters(&trimmed[index + 2..])
                {
                    let content_and_rest = &trimmed[index + 2 + content_start..];
                    let close_index = content_and_rest.find(&closing)?;
                    let end = index + 2 + content_start + close_index + closing.len();
                    if top_level && trimmed[end..].trim().is_empty() {
                        source = &trimmed[..index];
                        removed_comment = true;
                        break;
                    }
                    block_comment_end = Some(end);
                    continue;
                }
                if top_level {
                    source = &trimmed[..index];
                    removed_comment = true;
                    break;
                }
                line_comment = true;
                continue;
            }

            match character {
                '\'' | '"' => quote = Some(character),
                '[' => {
                    if let Some((content_start, closing)) =
                        parse_lua_long_bracket_delimiters(&trimmed[index..])
                    {
                        let content_and_rest = &trimmed[index + content_start..];
                        long_bracket_end = Some(
                            content_and_rest
                                .find(&closing)
                                .map_or(trimmed.len(), |close_index| {
                                    index + content_start + close_index + closing.len()
                                }),
                        );
                    } else {
                        bracket_depth = bracket_depth.saturating_add(1);
                    }
                }
                ']' => bracket_depth = bracket_depth.checked_sub(1)?,
                '{' => brace_depth = brace_depth.saturating_add(1),
                '}' => brace_depth = brace_depth.checked_sub(1)?,
                '(' => paren_depth = paren_depth.saturating_add(1),
                ')' => paren_depth = paren_depth.checked_sub(1)?,
                _ => {}
            }
        }

        if !removed_comment {
            return Some(trimmed);
        }
    }
}
