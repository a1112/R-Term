#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_dynamic_tab_title_text_return_from_statement(
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
    let rest = lua_trim_end_statement_separator(rest);
    let static_source = LuaStaticSource {
        source,
        max_start: start,
    };
    let parts = lua_tab_title_text_parts_from_expression(
        rest,
        tab_param,
        tabs_param,
        panes_param,
        Some(static_source),
        outer_static_source,
    )?;
    Some(NativeLuaTabTitle::Concat(parts))
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn native_lua_format_item_from_lua_table_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
) -> Option<NativeLuaFormatItem> {
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
        if key != "Text" || item.is_some() {
            return None;
        }
        item = Some(NativeLuaFormatItem::Text(
            lua_tab_title_text_parts_from_expression(
                value.trim(),
                tab_param,
                tabs_param,
                panes_param,
                static_source,
                outer_static_source,
            )?,
        ));
    }

    item
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_tab_title_text_parts_from_expression(
    expression: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
    lua_tab_title_text_parts_from_expression_with_depth(
        expression,
        tab_param,
        tabs_param,
        panes_param,
        static_source,
        outer_static_source,
        0,
    )
}

const LUA_TAB_TITLE_PARSE_MAX_DEPTH: usize = 16;

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_tab_title_text_parts_from_expression_with_depth(
    expression: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }
    let expression = lua_trim_start_comments(expression.trim())?;
    if let Some(part) = lua_tab_title_text_part_from_expression(
        expression,
        tab_param,
        tabs_param,
        panes_param,
        static_source,
        outer_static_source,
    ) {
        return Some(vec![part]);
    }

    if let Some(parts) = lua_tab_title_truncate_parts_from_expression(
        expression,
        tab_param,
        tabs_param,
        panes_param,
        static_source,
        outer_static_source,
        depth + 1,
    ) {
        return Some(parts);
    }

    if let Some(static_source) = static_source
        && let Some(lookup_static_source) =
            lua_static_source_before_current_statement(static_source, expression)
                .or(Some(static_source))
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            lookup_static_source.source,
            expression,
            lookup_static_source.max_start,
        )
        && let Some(parts) = lua_tab_title_text_parts_from_expression_with_depth(
            value,
            tab_param,
            tabs_param,
            panes_param,
            Some(lookup_static_source),
            outer_static_source,
            depth + 1,
        )
    {
        return Some(parts);
    }

    if let Some(parts) = lua_tab_title_helper_call_parts_from_expression(
        expression,
        tab_param,
        tabs_param,
        panes_param,
        outer_static_source,
        depth + 1,
    ) {
        return Some(parts);
    }

    if !expression.contains("..") {
        return None;
    }

    let mut parts = Vec::new();
    let mut has_dynamic_part = false;
    for segment in split_lua_string_concat_segments(expression)? {
        let segment = lua_trim_start_comments(segment.trim())?;
        let segment = lua_trim_end_statement_separator(segment);
        if let Some(part) = lua_tab_title_text_part_from_expression(
            segment,
            tab_param,
            tabs_param,
            panes_param,
            static_source,
            outer_static_source,
        ) {
            has_dynamic_part = true;
            parts.push(part);
            continue;
        }
        if let Some(segment_parts) = lua_tab_title_text_parts_from_expression_with_depth(
            segment,
            tab_param,
            tabs_param,
            panes_param,
            static_source,
            outer_static_source,
            depth + 1,
        ) {
            has_dynamic_part = true;
            parts.extend(segment_parts);
            continue;
        }
        let value =
            lua_static_string_value_from_expression(static_source, outer_static_source, segment)?;
        parts.push(NativeLuaTabTitleTextPart::Static(value));
    }

    has_dynamic_part.then_some(parts)
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_tab_title_truncate_parts_from_expression(
    expression: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    let (direction, rest) = lua_tab_title_truncate_direction_and_call_rest_from_expression(
        expression,
        static_source,
        outer_static_source,
    )?;
    let rest = lua_trim_start_comments(rest)?.strip_prefix('(')?;
    let (argument_list, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }

    let arguments = split_lua_top_level_arguments(argument_list)?;
    let [title_expression, width_expression] = arguments.as_slice() else {
        return None;
    };
    let lookup_static_source = static_source
        .and_then(|source| lua_static_source_before_current_statement(source, expression))
        .or(static_source);
    let parts = lua_tab_title_text_parts_from_expression_with_depth(
        title_expression.trim(),
        tab_param,
        tabs_param,
        panes_param,
        lookup_static_source,
        outer_static_source,
        depth + 1,
    )?;
    let max_width_offset = lua_tab_title_truncate_right_max_width_offset(width_expression.trim())?;

    Some(vec![match direction {
        NativeLuaTabTitleTruncateDirection::Left => NativeLuaTabTitleTextPart::TruncateLeft {
            parts,
            max_width_offset,
        },
        NativeLuaTabTitleTruncateDirection::Right => NativeLuaTabTitleTextPart::TruncateRight {
            parts,
            max_width_offset,
        },
    }])
}

fn lua_tab_title_truncate_direction_and_call_rest_from_expression<'a>(
    expression: &'a str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<(NativeLuaTabTitleTruncateDirection, &'a str)> {
    let expression = lua_trim_start_comments(expression.trim())?;
    let rest = lua_static_wezterm_receiver_rest_from_expression(
        expression,
        static_source,
        outer_static_source,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    let (field, rest) = lua_table_map_field_key_from_query_with_static_sources(
        static_source,
        outer_static_source,
        rest,
    )?;
    let direction = match field.as_str() {
        "truncate_left" => NativeLuaTabTitleTruncateDirection::Left,
        "truncate_right" => NativeLuaTabTitleTruncateDirection::Right,
        _ => return None,
    };
    Some((direction, rest))
}

fn lua_static_source_before_current_statement<'a>(
    static_source: LuaStaticSource<'a>,
    expression: &str,
) -> Option<LuaStaticSource<'a>> {
    let expression_start = lua_source_slice_start_offset(static_source.source, expression)?;
    let max_start = lua_top_level_statement_start_indices_before_offset(
        static_source.source,
        expression_start,
    )?
    .into_iter()
    .last()
    .unwrap_or(static_source.max_start.min(expression_start));
    Some(LuaStaticSource {
        source: static_source.source,
        max_start,
    })
}

fn lua_tab_title_truncate_right_max_width_offset(expression: &str) -> Option<usize> {
    let expression = lua_trim_start_comments(expression.trim())?;
    let rest = expression.strip_prefix("max_width")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    if lua_static_identifier_value_rest_is_statement_end(rest) {
        return Some(0);
    }

    let rest = lua_trim_start_comments(rest.strip_prefix('-')?)?;
    let offset = lua_unsigned_integer_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(offset.len()..)?)?;
    lua_static_identifier_value_rest_is_statement_end(rest).then(|| offset.parse().ok())?
}

#[expect(
    clippy::similar_names,
    reason = "singular and plural names mirror distinct compatibility API parameters"
)]
fn lua_tab_title_helper_call_parts_from_expression(
    expression: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
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

    lua_static_tab_title_helper_function_parts_before_offset(
        outer_static_source.source,
        function_name,
        outer_static_source.max_start,
        outer_static_source,
        tabs_param,
        panes_param,
        depth + 1,
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_static_tab_title_helper_function_parts_before_offset(
    source: &str,
    function_name: &str,
    max_start: usize,
    outer_static_source: LuaStaticSource<'_>,
    tabs_param: &str,
    panes_param: &str,
    depth: usize,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

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

        if lua_block_depth == 0
            && table_depth == 0
            && !character.is_whitespace()
            && lua_source_index_starts_statement(source, index)
            && let Some(statement) = lua_top_level_function_statement_from_index(source, index)
            && let Some(parts) = lua_tab_title_helper_function_parts_from_statement(
                statement,
                function_name,
                outer_static_source,
                tabs_param,
                panes_param,
                depth + 1,
            )
        {
            selected = Some(parts);
        }

        if lua_source_keyword_at(source, index, "elseif") {
            lua_block_depth = lua_block_depth.saturating_sub(1);
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
        }
    }

    selected
}

fn lua_top_level_function_statement_from_index(source: &str, index: usize) -> Option<&str> {
    if lua_source_keyword_at(source, index, "function") {
        return source.get(index..);
    }

    if !lua_source_keyword_at(source, index, "local") {
        return None;
    }
    let rest = lua_trim_start_comments(source.get(index + "local".len()..)?)?;
    lua_source_keyword_at(rest, 0, "function").then_some(rest)
}

fn lua_tab_title_helper_function_parts_from_statement(
    statement: &str,
    function_name: &str,
    outer_static_source: LuaStaticSource<'_>,
    tabs_param: &str,
    panes_param: &str,
    depth: usize,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
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
    lua_tab_title_return_text_parts_from_function_body(
        body,
        first_param,
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
fn lua_tab_title_return_text_parts_from_function_body(
    body: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    if let Some(parts) = lua_tab_title_explicit_title_fallback_parts_from_function_body(
        body,
        tab_param,
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
        if let Some(parts) = lua_tab_title_text_parts_from_expression_with_depth(
            expression,
            tab_param,
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
fn lua_tab_title_explicit_title_fallback_parts_from_function_body(
    body: &str,
    tab_param: &str,
    tabs_param: &str,
    panes_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    depth: usize,
) -> Option<Vec<NativeLuaTabTitleTextPart>> {
    if depth > LUA_TAB_TITLE_PARSE_MAX_DEPTH {
        return None;
    }

    let starts = lua_top_level_statement_start_indices_before_offset(body, body.len())?;
    for (position, start) in starts.iter().enumerate() {
        let statement = lua_trim_start_comments(body.get(*start..)?)?;
        let Some((condition, if_body)) =
            lua_static_if_condition_and_body_branches_from_statement(statement)
                .and_then(|(branches, _)| branches.first().copied())
        else {
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
            lua_tab_title_text_part_from_expression(
                assigned_value,
                tab_param,
                tabs_param,
                panes_param,
                Some(if_static_source),
                outer_static_source,
            ),
            Some(NativeLuaTabTitleTextPart::ActiveTabTitle)
        ) {
            continue;
        }

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
            let Some(fallback_parts) = lua_tab_title_text_parts_from_expression_with_depth(
                fallback_expression,
                tab_param,
                tabs_param,
                panes_param,
                Some(fallback_static_source),
                outer_static_source,
                depth + 1,
            ) else {
                continue;
            };
            if matches!(
                fallback_parts.as_slice(),
                [NativeLuaTabTitleTextPart::ActivePaneTitle]
            ) {
                return Some(vec![
                    NativeLuaTabTitleTextPart::ActiveTabTitleOrActivePaneTitle,
                ]);
            }
        }
    }

    None
}

fn lua_static_return_identifier_from_function_body(body: &str) -> Option<&str> {
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let Some(expression) = lua_static_return_expression_from_statement(statement) else {
            continue;
        };
        let Some(identifier) = lua_identifier_literal_from_query(expression) else {
            continue;
        };
        if lua_static_identifier_value_rest_is_statement_end(expression.get(identifier.len()..)?) {
            return Some(identifier);
        }
    }

    None
}

fn lua_static_return_expression_from_statement(statement: &str) -> Option<&str> {
    let rest = statement.strip_prefix("return")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?;
    Some(lua_trim_end_statement_separator(rest))
}

fn lua_static_if_condition_and_body_branches_from_statement(
    statement: &str,
) -> Option<(Vec<(&str, &str)>, &str)> {
    let (branches, else_body, rest) =
        lua_static_if_condition_and_body_branches_and_else_from_statement(statement)?;
    if else_body.is_some() {
        return None;
    }
    Some((branches, rest))
}

#[expect(
    clippy::type_complexity,
    reason = "tuple shape mirrors the compatibility data contract"
)]
fn lua_static_if_condition_and_body_branches_and_else_from_statement(
    statement: &str,
) -> Option<(Vec<(&str, &str)>, Option<&str>, &str)> {
    let statement = lua_trim_start_comments(statement)?;
    if !lua_source_keyword_at(statement, 0, "if") {
        return None;
    }
    let mut branches = Vec::new();
    let rest = lua_trim_start_comments(statement.get("if".len()..)?)?;
    let then = lua_static_if_then_index_from_query(rest)?;
    let condition = rest.get(..then)?.trim();
    let body_start = lua_trim_start_comments(rest.get(then + "then".len()..)?)?;
    let (body, mut rest) = lua_static_if_branch_body_and_rest_from_query(body_start)?;
    branches.push((condition, body));

    loop {
        rest = lua_trim_start_comments(rest)?;
        if !lua_source_keyword_at(rest, 0, "elseif") {
            break;
        }
        let branch = lua_trim_start_comments(rest.get("elseif".len()..)?)?;
        let then = lua_static_if_then_index_from_query(branch)?;
        let condition = branch.get(..then)?.trim();
        let body_start = lua_trim_start_comments(branch.get(then + "then".len()..)?)?;
        let (body, branch_rest) = lua_static_if_branch_body_and_rest_from_query(body_start)?;
        branches.push((condition, body));
        rest = branch_rest;
    }

    let mut rest = lua_trim_start_comments(rest)?;
    let else_body = if lua_source_keyword_at(rest, 0, "else") {
        let body_start = lua_trim_start_comments(rest.get("else".len()..)?)?;
        let (body, branch_rest) = lua_static_if_branch_body_and_rest_from_query(body_start)?;
        rest = lua_trim_start_comments(branch_rest)?;
        Some(body)
    } else {
        None
    };

    if !lua_source_keyword_at(rest, 0, "end") {
        return None;
    }
    Some((branches, else_body, rest.get("end".len()..)?))
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_static_if_branch_body_and_rest_from_query(value: &str) -> Option<(&str, &str)> {
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    let mut lua_block_depth = 0usize;
    let mut table_depth = 0usize;

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

        if lua_block_depth == 0
            && table_depth == 0
            && (lua_source_keyword_at(value, index, "elseif")
                || lua_source_keyword_at(value, index, "else")
                || lua_source_keyword_at(value, index, "end"))
        {
            return Some((value.get(..index)?.trim(), value.get(index..)?));
        }

        if lua_source_keyword_at(value, index, "end")
            || lua_source_keyword_at(value, index, "until")
        {
            lua_block_depth = lua_block_depth.saturating_sub(1);
        }
    }

    None
}

fn lua_static_if_then_index_from_query(value: &str) -> Option<usize> {
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
            _ => {}
        }

        if lua_source_keyword_at(value, index, "then") {
            return Some(index);
        }
    }

    None
}

fn lua_tab_title_non_empty_condition_variable(condition: &str) -> Option<&str> {
    let condition = lua_trim_start_comments(condition.trim())?;
    let variable = lua_identifier_literal_from_query(condition)?;
    let rest = condition.get(variable.len()..)?;
    let rest = lua_trim_start_comments(rest)?;
    let rest = rest.strip_prefix("and")?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }

    let rest = lua_trim_start_comments(rest)?.strip_prefix('#')?;
    let rest = lua_trim_start_comments(rest)?;
    let length_variable = lua_identifier_literal_from_query(rest)?;
    if length_variable != variable {
        return None;
    }

    let rest = lua_trim_start_comments(rest.get(length_variable.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix('>')?)?;
    let rest = rest.strip_prefix('0')?;
    rest.trim().is_empty().then_some(variable)
}

fn lua_static_tab_title_return_from_statement(
    statement: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeTabTitle> {
    lua_static_string_return_from_statement(statement)
        .map(NativeTabTitle::Text)
        .or_else(|| {
            lua_static_string_variable_return_from_statement(
                statement,
                static_source,
                outer_static_source,
            )
            .map(NativeTabTitle::Text)
        })
        .or_else(|| {
            lua_static_string_concat_return_from_statement(
                static_source.source,
                static_source.max_start,
                statement,
                outer_static_source,
            )
            .map(NativeTabTitle::Text)
        })
        .or_else(|| {
            let rest = statement.strip_prefix("return")?;
            if rest.chars().next().is_some_and(is_lua_identifier_character) {
                return None;
            }
            let rest = lua_trim_start_comments(rest)?;
            let rest = rest.strip_suffix(';').unwrap_or(rest).trim();
            if let Some(items) = native_format_items_from_lua_format_items_table_query(rest) {
                return Some(NativeTabTitle::Format(items));
            }
            if let Some(items) = native_format_items_from_wezterm_format_query_with_static_sources(
                Some(static_source),
                outer_static_source,
                rest,
            ) {
                return Some(NativeTabTitle::Format(items));
            }

            let variable = lua_identifier_literal_from_query(rest)?;
            let variable_rest = rest.get(variable.len()..)?;
            if !lua_static_identifier_value_rest_is_statement_end(variable_rest) {
                return None;
            }
            if let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
                static_source.source,
                rest,
                static_source.max_start,
            )
                && let Some(items) = native_format_items_from_wezterm_format_query_with_static_sources(
                    Some(static_source),
                    outer_static_source,
                    value,
                )
            {
                return Some(NativeTabTitle::Format(items));
            }
            if let Some(outer_static_source) = outer_static_source
                && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
                    outer_static_source.source,
                    rest,
                    outer_static_source.max_start,
                )
                && let Some(items) = native_format_items_from_wezterm_format_query_with_static_sources(
                    None,
                    Some(outer_static_source),
                    value,
                )
            {
                return Some(NativeTabTitle::Format(items));
            }
            if let Some(items) =
                native_format_items_from_static_lua_table_variable(
                    static_source,
                    outer_static_source,
                    variable,
                )
            {
                return items.map(NativeTabTitle::Format);
            }
            if let Some(outer_static_source) = outer_static_source
                && let Some(items) = native_format_items_from_static_lua_table_variable(
                    outer_static_source,
                    None,
                    variable,
                )
            {
                return items.map(NativeTabTitle::Format);
            }
            None
        })
}

fn lua_static_status_update_from_function_body(
    body: &str,
    window_name: &str,
    pane_name: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowStatusUpdate> {
    let mut update = NativeLuaWindowStatusUpdate {
        left_status: None,
        right_status: None,
    };

    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let static_source = LuaStaticSource {
            source: body,
            max_start: start,
        };
        if let Some(left_status) = lua_static_window_status_setter_from_statement(
            statement,
            Some(static_source),
            outer_static_source,
            window_name,
            pane_name,
            "set_left_status",
        ) {
            update.left_status = Some(left_status);
        }
        if let Some(right_status) = lua_static_window_status_setter_from_statement(
            statement,
            Some(static_source),
            outer_static_source,
            window_name,
            pane_name,
            "set_right_status",
        ) {
            update.right_status = Some(right_status);
        }
    }

    (update.left_status.is_some() || update.right_status.is_some()).then_some(update)
}

fn lua_static_window_config_overrides_from_function_body(
    body: &str,
    window_name: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeWindowConfigPatch> {
    let mut overrides = NativeWindowConfigPatch::default();

    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let static_source = LuaStaticSource {
            source: body,
            max_start: start,
        };
        if let Some(update) = lua_static_window_config_overrides_from_statement(
            statement,
            static_source,
            outer_static_source,
            window_name,
        ) {
            overrides.merge(update);
        }
    }

    (!overrides.is_empty()).then_some(overrides)
}

fn lua_static_window_config_overrides_from_statement(
    statement: &str,
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
) -> Option<NativeWindowConfigPatch> {
    let rest = statement.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = "set_config_overrides";
    if !rest.starts_with(method) || !lua_config_assignment_field_has_boundaries(rest, 0, method) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let argument = if let Some(rest) = rest.strip_prefix('(') {
        let (arguments, _) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        let arguments = split_lua_top_level_arguments(arguments)?;
        let [argument] = arguments.as_slice() else {
            return None;
        };
        *argument
    } else {
        lua_top_level_statement_value_from_query(rest)?
    };
    lua_static_window_config_overrides_from_query(
        argument,
        Some(static_source),
        outer_static_source,
    )
}

fn lua_static_window_config_overrides_from_query(
    argument: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeWindowConfigPatch> {
    let argument = lua_trim_start_comments(argument)?;
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            argument,
            static_source.max_start,
        )
    {
        return lua_static_window_config_overrides_from_query(
            value,
            Some(static_source),
            outer_static_source,
        );
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            outer_static_source.source,
            argument,
            outer_static_source.max_start,
        )
    {
        return lua_static_window_config_overrides_from_query(
            value,
            static_source,
            Some(outer_static_source),
        );
    }
    let table = argument.trim();
    if !table.starts_with('{') {
        return None;
    }
    let config = format!("return {table}");
    let overrides = native_config_overrides_from_wezterm_lua_config(&config)?;
    native_window_config_patch_from_snapshot(overrides)
}

fn native_window_config_patch_from_snapshot(
    overrides: NativeConfigSnapshot,
) -> Option<NativeWindowConfigPatch> {
    let mut values = NativeWindowConfigPatchValues::default();
    overrides.write_patch_values(&mut values);
    Some(NativeWindowConfigPatch::from_values(values)).filter(|overrides| !overrides.is_empty())
}

fn lua_static_user_var_changed_from_function_body(
    body: &str,
    window_name: &str,
    pane_name: &str,
    name_param: &str,
    value_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaUserVarChanged> {
    let mut update = NativeLuaUserVarChanged {
        left_status: None,
        right_status: None,
    };

    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        let static_source = LuaStaticSource {
            source: body,
            max_start: start,
        };
        if let Some(left_status) = lua_static_user_var_changed_status_setter_from_statement(
            statement,
            static_source,
            window_name,
            pane_name,
            name_param,
            value_param,
            outer_static_source,
            "set_left_status",
        ) {
            update.left_status = Some(left_status);
        }
        if let Some(right_status) = lua_static_user_var_changed_status_setter_from_statement(
            statement,
            static_source,
            window_name,
            pane_name,
            name_param,
            value_param,
            outer_static_source,
            "set_right_status",
        ) {
            update.right_status = Some(right_status);
        }
    }

    (update.left_status.is_some() || update.right_status.is_some()).then_some(update)
}

#[expect(
    clippy::too_many_arguments,
    reason = "compatibility operation requires the complete evaluation context"
)]
fn lua_static_user_var_changed_status_setter_from_statement(
    statement: &str,
    static_source: LuaStaticSource<'_>,
    window_name: &str,
    pane_name: &str,
    name_param: &str,
    value_param: &str,
    outer_static_source: Option<LuaStaticSource<'_>>,
    method: &str,
) -> Option<NativeLuaUserVarChangedStatusText> {
    let rest = statement.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with(method) || !lua_config_assignment_field_has_boundaries(rest, 0, method) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?.strip_prefix('(')?;
    let (arguments, _) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    let arguments = split_lua_top_level_arguments(arguments)?;
    let [argument] = arguments.as_slice() else {
        return None;
    };
    if let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        argument,
        static_source.max_start,
    ) {
        return lua_static_user_var_changed_status_text_from_query(
            value,
            window_name,
            pane_name,
            name_param,
            value_param,
            Some(static_source),
            outer_static_source,
        );
    }
    lua_static_user_var_changed_status_text_from_query(
        argument,
        window_name,
        pane_name,
        name_param,
        value_param,
        Some(static_source),
        outer_static_source,
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_static_user_var_changed_status_text_from_query(
    value: &str,
    window_name: &str,
    pane_name: &str,
    name_param: &str,
    value_param: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaUserVarChangedStatusText> {
    let user_vars_variable = static_source.and_then(|static_source| {
        lua_static_pane_user_vars_variable_source_before_offset(
            static_source.source,
            window_name,
            pane_name,
            static_source.max_start,
        )
    });
    let mut parts = Vec::new();
    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        let segment = lua_tostring_argument_from_query(segment).unwrap_or(segment);
        if lua_static_identifier_expression_matches(segment, name_param) {
            parts.push(NativeLuaUserVarChangedStatusPart::Name);
        } else if lua_static_identifier_expression_matches(segment, value_param) {
            parts.push(NativeLuaUserVarChangedStatusPart::Value);
        } else if let Some(part) = lua_static_user_var_changed_event_param_fallback_from_query(
            static_source,
            outer_static_source,
            segment,
            name_param,
            value_param,
        ) {
            parts.push(part);
        } else if let Some(part) = static_source.and_then(|static_source| {
            lua_static_user_var_changed_local_event_param_from_query(
                static_source,
                outer_static_source,
                segment,
                name_param,
                value_param,
            )
        }) {
            parts.push(part);
        } else if lua_window_zero_arg_method_name_from_query(segment, window_name)
            == Some("window_id")
        {
            parts.push(NativeLuaUserVarChangedStatusPart::WindowId);
        } else if lua_window_zero_arg_method_name_from_query(segment, pane_name) == Some("pane_id")
        {
            parts.push(NativeLuaUserVarChangedStatusPart::PaneId);
        } else if let Some(part) = static_source.and_then(|static_source| {
            lua_static_user_var_changed_local_window_pane_id_from_query(
                static_source,
                segment,
                window_name,
                pane_name,
            )
        }) {
            parts.push(part);
        } else if let Some((source, name, fallback)) = static_source.and_then(|static_source| {
            lua_static_pane_user_var_source_fallback_from_query(
                static_source,
                outer_static_source,
                segment,
                user_vars_variable.as_ref(),
                window_name,
                pane_name,
            )
        }) {
            parts.push(NativeLuaUserVarChangedStatusPart::PaneUserVar {
                source,
                name,
                fallback,
            });
        } else if let Some((source, name)) = lua_static_pane_user_var_source_name_from_query(
            static_source,
            outer_static_source,
            segment,
            user_vars_variable.as_ref(),
            window_name,
            pane_name,
        ) {
            parts.push(NativeLuaUserVarChangedStatusPart::PaneUserVar {
                source,
                name,
                fallback: String::new(),
            });
        } else if let Some((source, name, fallback)) = static_source.and_then(|static_source| {
            lua_static_user_var_changed_local_pane_user_var_from_query(
                static_source,
                outer_static_source,
                segment,
                user_vars_variable.as_ref(),
                window_name,
                pane_name,
            )
        }) {
            parts.push(NativeLuaUserVarChangedStatusPart::PaneUserVar {
                source,
                name,
                fallback,
            });
        } else if let Some(text) =
            lua_static_string_value_from_expression(static_source, outer_static_source, segment)
        {
            parts.push(NativeLuaUserVarChangedStatusPart::Static(text));
        } else {
            return None;
        }
    }
    (!parts.is_empty()).then_some(NativeLuaUserVarChangedStatusText { parts })
}

fn lua_static_user_var_changed_event_param_fallback_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    name_param: &str,
    value_param: &str,
) -> Option<NativeLuaUserVarChangedStatusPart> {
    let (dynamic, fallback) = lua_dynamic_status_fallback_from_query(value)?;
    lua_static_string_value_from_expression(static_source, outer_static_source, fallback)?;
    if lua_static_identifier_expression_matches(dynamic, name_param) {
        Some(NativeLuaUserVarChangedStatusPart::Name)
    } else if lua_static_identifier_expression_matches(dynamic, value_param) {
        Some(NativeLuaUserVarChangedStatusPart::Value)
    } else {
        None
    }
}

fn lua_static_user_var_changed_local_window_pane_id_from_query(
    static_source: LuaStaticSource<'_>,
    value: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<NativeLuaUserVarChangedStatusPart> {
    let value = lua_tostring_argument_from_query(value).unwrap_or(value);
    let local_value = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
    )?;
    if lua_window_zero_arg_method_name_from_query(local_value, window_name) == Some("window_id") {
        Some(NativeLuaUserVarChangedStatusPart::WindowId)
    } else if lua_window_zero_arg_method_name_from_query(local_value, pane_name) == Some("pane_id")
    {
        Some(NativeLuaUserVarChangedStatusPart::PaneId)
    } else {
        None
    }
}

fn lua_static_user_var_changed_local_event_param_from_query(
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    name_param: &str,
    value_param: &str,
) -> Option<NativeLuaUserVarChangedStatusPart> {
    let value = lua_tostring_argument_from_query(value).unwrap_or(value);
    let local_value = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
    )?;
    let local_value = lua_tostring_argument_from_query(local_value).unwrap_or(local_value);
    let local_value =
        if let Some((dynamic, fallback)) = lua_dynamic_status_fallback_from_query(local_value) {
            lua_static_string_value_from_expression(
                Some(static_source),
                outer_static_source,
                fallback,
            )?;
            dynamic
        } else {
            local_value
        };
    if lua_static_identifier_expression_matches(local_value, name_param) {
        Some(NativeLuaUserVarChangedStatusPart::Name)
    } else if lua_static_identifier_expression_matches(local_value, value_param) {
        Some(NativeLuaUserVarChangedStatusPart::Value)
    } else {
        None
    }
}

fn lua_static_user_var_changed_local_pane_user_var_from_query(
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    variable: Option<&(String, NativeLuaUserVarChangedPaneUserVarSource)>,
    window_name: &str,
    pane_name: &str,
) -> Option<(NativeLuaUserVarChangedPaneUserVarSource, String, String)> {
    let local_value = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
    )?;
    if let Some((source, name, fallback)) = lua_static_pane_user_var_source_fallback_from_query(
        static_source,
        outer_static_source,
        local_value,
        variable,
        window_name,
        pane_name,
    ) {
        return Some((source, name, fallback));
    }
    lua_static_pane_user_var_source_name_from_query(
        Some(static_source),
        outer_static_source,
        local_value,
        variable,
        window_name,
        pane_name,
    )
    .map(|(source, name)| (source, name, String::new()))
}

fn lua_static_identifier_expression_matches(value: &str, expected: &str) -> bool {
    let value = lua_trim_start_comments(value).unwrap_or(value).trim();
    let Some(rest) = value.strip_prefix(expected) else {
        return false;
    };
    !rest.chars().next().is_some_and(is_lua_identifier_character)
        && lua_trim_start_comments(rest).is_some_and(str::is_empty)
}

fn lua_static_window_status_setter_from_statement(
    statement: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    pane_name: &str,
    method: &str,
) -> Option<NativeLuaWindowStatusText> {
    let rest = statement.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    if !rest.starts_with(method) || !lua_config_assignment_field_has_boundaries(rest, 0, method) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    if let Some(rest) = rest.strip_prefix('(') {
        let (arguments, _) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        let arguments = split_lua_top_level_arguments(arguments)?;
        let [argument] = arguments.as_slice() else {
            return None;
        };
        return lua_static_window_status_text_from_parenthesized_argument(
            static_source,
            outer_static_source,
            window_name,
            pane_name,
            argument,
        );
    }
    lua_inline_string_literal_value_and_len(rest)
        .map(|(status, _)| NativeLuaWindowStatusText::Static(status))
}

fn lua_static_window_status_text_from_parenthesized_argument(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    pane_name: &str,
    argument: &str,
) -> Option<NativeLuaWindowStatusText> {
    let argument = lua_trim_start_comments(argument)?;
    if let Some(static_source) = static_source
        && let Some(status) = lua_static_window_status_variable_text_from_query(
            static_source,
            window_name,
            pane_name,
            argument,
        )
    {
        return Some(status);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            static_source.source,
            argument,
            static_source.max_start,
        )
    {
        return lua_static_window_status_text_from_query(
            Some(static_source),
            outer_static_source,
            window_name,
            pane_name,
            value,
        );
    }
    if let Some(outer_static_source) = outer_static_source
        && let Some(value) = lua_static_expression_assignment_value_before_offset_from_query(
            outer_static_source.source,
            argument,
            outer_static_source.max_start,
        )
    {
        return lua_static_window_status_text_from_query(
            None,
            Some(outer_static_source),
            window_name,
            pane_name,
            value,
        );
    }
    lua_static_window_status_text_from_query(
        static_source,
        outer_static_source,
        window_name,
        pane_name,
        argument,
    )
}

fn lua_static_window_status_text_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let argument = lua_trim_start_comments(value)?;
    let argument = lua_tostring_argument_from_query(argument).unwrap_or(argument);
    if let Some(status) =
        lua_static_string_value_from_expression(static_source, outer_static_source, argument)
    {
        return Some(NativeLuaWindowStatusText::Static(status));
    }
    if let Some(static_source) = static_source
        && let Some(status) = lua_static_keyboard_modifiers_status_text_from_query(
            static_source,
            window_name,
            argument,
        )
    {
        return Some(status);
    }
    if let Some(static_source) = static_source
        && let Some(status) = lua_static_window_dimensions_status_text_from_query(
            static_source,
            window_name,
            argument,
        )
    {
        return Some(status);
    }
    if let Some(status) = lua_static_window_effective_config_status_text_from_query(
        static_source,
        outer_static_source,
        window_name,
        argument,
    ) {
        return Some(status);
    }
    if let Some(static_source) = static_source
        && let Some(status) = lua_static_pane_dimensions_status_text_from_query(
            static_source,
            window_name,
            pane_name,
            argument,
        )
    {
        return Some(status);
    }
    if let Some(static_source) = static_source
        && let Some(status) = lua_static_pane_cursor_position_status_text_from_query(
            static_source,
            window_name,
            pane_name,
            argument,
        )
    {
        return Some(status);
    }
    if let Some(static_source) = static_source
        && let Some(status) = lua_static_pane_user_vars_status_text_from_query(
            static_source,
            outer_static_source,
            window_name,
            pane_name,
            argument,
        )
    {
        return Some(status);
    }
    if let Some(status) = lua_static_window_id_status_text_from_query(window_name, argument) {
        return Some(status);
    }
    if let Some(status) = lua_static_window_and_pane_status_text_from_query(
        static_source,
        outer_static_source,
        window_name,
        pane_name,
        argument,
    ) {
        return Some(status);
    }
    if let Some(status) = lua_static_window_and_pane_status_fallback_text_from_query(
        static_source,
        outer_static_source,
        window_name,
        pane_name,
        argument,
    ) {
        return Some(status);
    }
    if let Some(status) = lua_window_status_method_text_from_query(argument, window_name) {
        return Some(status);
    }
    wezterm_format_status_text_from_query(static_source, outer_static_source, argument)
        .map(NativeLuaWindowStatusText::Static)
}

fn lua_static_window_id_status_text_from_query(
    window_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut has_window_id = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        if lua_window_zero_arg_method_name_from_query(segment, window_name) == Some("window_id") {
            if has_window_id {
                return None;
            }
            has_window_id = true;
            continue;
        }
        let text = lua_static_string_value_from_expression(None, None, segment)?;
        if has_window_id {
            suffix.push_str(&text);
        } else {
            prefix.push_str(&text);
        }
    }

    has_window_id.then_some(NativeLuaWindowStatusText::WindowId { prefix, suffix })
}

fn lua_static_window_and_pane_status_fallback_text_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let part = lua_static_window_and_pane_status_fallback_part_from_query(
        static_source,
        outer_static_source,
        window_name,
        pane_name,
        value,
    )?;
    Some(NativeLuaWindowStatusText::WindowPane { parts: vec![part] })
}

fn lua_static_window_and_pane_status_fallback_part_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowPaneStatusPart> {
    let (dynamic, fallback) = lua_dynamic_status_fallback_from_query(value)?;
    lua_static_string_value_from_expression(static_source, outer_static_source, fallback)?;
    static_source
        .and_then(|static_source| {
            lua_static_window_and_pane_status_part_receiver_alias_from_query(
                static_source,
                dynamic,
                window_name,
            )
        })
        .or_else(|| {
            static_source.and_then(|static_source| {
                lua_static_window_and_pane_status_part_variable_from_query(
                    static_source,
                    dynamic,
                    window_name,
                    pane_name,
                )
            })
        })
        .or_else(|| {
            lua_static_window_and_pane_status_part_from_query(dynamic, window_name, pane_name)
        })
}

fn lua_dynamic_status_fallback_from_query(value: &str) -> Option<(&str, &str)> {
    let value = lua_trim_start_comments(value)?;
    let value = lua_parenthesized_lua_expression_from_query(value).unwrap_or(value);
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
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
                table_depth = table_depth.checked_sub(1)?;
                continue;
            }
            '(' => {
                paren_depth = paren_depth.saturating_add(1);
                continue;
            }
            ')' => {
                paren_depth = paren_depth.checked_sub(1)?;
                continue;
            }
            _ => {}
        }

        if table_depth == 0 && paren_depth == 0 && lua_source_keyword_at(value, index, "or") {
            let dynamic = value[..index].trim();
            let fallback = value[index + "or".len()..].trim();
            return (!dynamic.is_empty() && !fallback.is_empty()).then_some((dynamic, fallback));
        }
    }

    None
}

fn lua_parenthesized_lua_expression_from_query(value: &str) -> Option<&str> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix('(')?;
    let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    lua_trim_start_comments(rest)?
        .is_empty()
        .then_some(lua_trim_start_comments(argument)?.trim())
}

fn lua_static_window_and_pane_status_text_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        let segment = lua_tostring_argument_from_query(segment).unwrap_or(segment);
        if let Some(part) =
            lua_static_window_and_pane_status_part_from_query(segment, window_name, pane_name)
                .or_else(|| {
                    lua_static_window_and_pane_status_fallback_part_from_query(
                        static_source,
                        outer_static_source,
                        window_name,
                        pane_name,
                        segment,
                    )
                })
                .or_else(|| {
                    static_source.and_then(|static_source| {
                        lua_static_window_and_pane_status_part_variable_from_query(
                            static_source,
                            segment,
                            window_name,
                            pane_name,
                        )
                    })
                })
        {
            parts.push(part);
            has_dynamic_part = true;
        } else if let Some(text) = lua_static_string_value_from_expression(None, None, segment) {
            parts.push(NativeLuaWindowPaneStatusPart::Static(text));
        } else {
            return None;
        }
    }

    has_dynamic_part.then_some(NativeLuaWindowStatusText::WindowPane { parts })
}

fn lua_static_window_and_pane_status_part_variable_from_query(
    static_source: LuaStaticSource<'_>,
    value: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<NativeLuaWindowPaneStatusPart> {
    if let Some(part) = lua_static_window_and_pane_status_part_receiver_alias_from_query(
        static_source,
        value,
        window_name,
    ) {
        return Some(part);
    }

    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let assignment = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )?;
    let assignment = lua_tostring_argument_from_query(assignment).unwrap_or(assignment);
    if let Some(part) = lua_static_window_and_pane_status_part_receiver_alias_from_query(
        static_source,
        assignment,
        window_name,
    ) {
        return Some(part);
    }
    lua_static_window_and_pane_status_part_from_query(assignment, window_name, pane_name)
}

fn lua_static_window_and_pane_status_part_receiver_alias_from_query(
    static_source: LuaStaticSource<'_>,
    value: &str,
    window_name: &str,
) -> Option<NativeLuaWindowPaneStatusPart> {
    let receiver = lua_identifier_literal_from_query(value)?;
    let method = lua_window_zero_arg_method_name_from_query(value, receiver)?;
    let assignment = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        receiver,
        static_source.max_start,
    )?;

    if lua_window_zero_arg_method_name_from_query(assignment, window_name) == Some("active_tab") {
        return match method {
            "tab_id" => Some(NativeLuaWindowPaneStatusPart::ActiveTabId),
            "get_title" => Some(NativeLuaWindowPaneStatusPart::ActiveTabTitle),
            _ => None,
        };
    }
    if lua_window_zero_arg_method_name_from_query(assignment, window_name) == Some("active_pane") {
        return match method {
            "pane_id" => Some(NativeLuaWindowPaneStatusPart::PaneId),
            "get_title" => Some(NativeLuaWindowPaneStatusPart::PaneTitle),
            "get_domain_name" => Some(NativeLuaWindowPaneStatusPart::PaneDomainName),
            "get_current_working_dir" => Some(NativeLuaWindowPaneStatusPart::PaneCurrentWorkingDir),
            "get_foreground_process_name" => {
                Some(NativeLuaWindowPaneStatusPart::PaneForegroundProcessName)
            }
            "get_tty_name" => Some(NativeLuaWindowPaneStatusPart::PaneTtyName),
            _ => None,
        };
    }
    None
}

fn lua_static_window_and_pane_status_part_from_query(
    value: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<NativeLuaWindowPaneStatusPart> {
    if lua_window_zero_arg_method_name_from_query(value, window_name) == Some("active_workspace") {
        return Some(NativeLuaWindowPaneStatusPart::ActiveWorkspace);
    }
    if lua_window_zero_arg_method_name_from_query(value, window_name) == Some("window_id") {
        return Some(NativeLuaWindowPaneStatusPart::WindowId);
    }
    if let Some(method) = lua_window_active_tab_zero_arg_method_name_from_query(value, window_name)
    {
        return match method {
            "tab_id" => Some(NativeLuaWindowPaneStatusPart::ActiveTabId),
            "get_title" => Some(NativeLuaWindowPaneStatusPart::ActiveTabTitle),
            _ => None,
        };
    }
    if let Some(method) = lua_window_active_pane_zero_arg_method_name_from_query(value, window_name)
    {
        return match method {
            "pane_id" => Some(NativeLuaWindowPaneStatusPart::PaneId),
            "get_title" => Some(NativeLuaWindowPaneStatusPart::PaneTitle),
            "get_domain_name" => Some(NativeLuaWindowPaneStatusPart::PaneDomainName),
            "get_current_working_dir" => Some(NativeLuaWindowPaneStatusPart::PaneCurrentWorkingDir),
            "get_foreground_process_name" => {
                Some(NativeLuaWindowPaneStatusPart::PaneForegroundProcessName)
            }
            "get_tty_name" => Some(NativeLuaWindowPaneStatusPart::PaneTtyName),
            _ => None,
        };
    }
    if lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("pane_id") {
        return Some(NativeLuaWindowPaneStatusPart::PaneId);
    }
    if lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("get_title") {
        return Some(NativeLuaWindowPaneStatusPart::PaneTitle);
    }
    if lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("get_domain_name") {
        return Some(NativeLuaWindowPaneStatusPart::PaneDomainName);
    }
    if lua_window_zero_arg_method_name_from_query(value, pane_name)
        == Some("get_current_working_dir")
    {
        return Some(NativeLuaWindowPaneStatusPart::PaneCurrentWorkingDir);
    }
    if lua_window_zero_arg_method_name_from_query(value, pane_name)
        == Some("get_foreground_process_name")
    {
        return Some(NativeLuaWindowPaneStatusPart::PaneForegroundProcessName);
    }
    if lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("get_tty_name") {
        return Some(NativeLuaWindowPaneStatusPart::PaneTtyName);
    }
    None
}

fn lua_tostring_argument_from_query(value: &str) -> Option<&str> {
    let value = lua_trim_start_comments(value)?.trim();
    if !value.starts_with("tostring")
        || !lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        return None;
    }
    let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    lua_trim_start_comments(rest)?
        .is_empty()
        .then_some(lua_trim_start_comments(argument)?.trim())
}

fn lua_static_window_dimensions_status_text_from_query(
    static_source: LuaStaticSource<'_>,
    window_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let variable = lua_static_window_dimensions_variable_before_offset(
        static_source.source,
        window_name,
        static_source.max_start,
    )?;
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        if let Some(field) = lua_static_window_dimensions_field_from_query(segment, &variable) {
            parts.push(NativeLuaWindowDimensionsStatusPart::Field(field));
            has_dynamic_part = true;
        } else if let Some(text) = lua_static_string_value_from_expression(None, None, segment) {
            parts.push(NativeLuaWindowDimensionsStatusPart::Static(text));
        } else {
            return None;
        }
    }

    has_dynamic_part.then_some(NativeLuaWindowStatusText::WindowDimensions { parts })
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_static_window_effective_config_status_text_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let variable = static_source
        .and_then(|static_source| {
            lua_static_window_effective_config_variable_before_offset(
                static_source.source,
                window_name,
                static_source.max_start,
            )
        })
        .or_else(|| {
            outer_static_source.and_then(|outer_static_source| {
                lua_static_window_effective_config_variable_before_offset(
                    outer_static_source.source,
                    window_name,
                    outer_static_source.max_start,
                )
            })
        });
    let palette_variable = static_source.and_then(|static_source| {
        lua_static_window_effective_config_resolved_palette_variable_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )
    });
    let visual_bell_variable = static_source.and_then(|static_source| {
        lua_static_window_effective_config_visual_bell_variable_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )
    });
    let environment_variable = static_source.and_then(|static_source| {
        lua_static_window_effective_config_environment_variable_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )
    });
    let cell_widths_variable = static_source.and_then(|static_source| {
        lua_static_window_effective_config_cell_widths_variable_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )
    });
    let hyperlink_rule_variable = static_source.and_then(|static_source| {
        lua_static_window_effective_config_hyperlink_rule_variable_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )
    });
    let launch_menu_variable = static_source.and_then(|static_source| {
        lua_static_window_effective_config_launch_menu_variable_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )
    });
    let launch_menu_env_variable = static_source.and_then(|static_source| {
        lua_static_window_effective_config_launch_menu_env_variable_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )
    });
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        if let Some(field) = lua_window_effective_config_field_from_query_with_static_sources(
            segment,
            window_name,
            static_source,
            outer_static_source,
        )
        .or_else(|| {
            variable.as_deref().and_then(|variable| {
                lua_static_window_effective_config_field_from_query_with_static_sources(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        })
        .or_else(|| {
            palette_variable.as_deref().and_then(|variable| {
                lua_static_window_effective_config_resolved_palette_field_from_query(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        })
        .or_else(|| {
            visual_bell_variable.as_deref().and_then(|variable| {
                lua_static_window_effective_config_visual_bell_field_from_query(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        })
        .or_else(|| {
            environment_variable.as_deref().and_then(|variable| {
                lua_static_window_effective_config_environment_field_from_query(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        })
        .or_else(|| {
            cell_widths_variable.as_ref().and_then(|variable| {
                lua_static_window_effective_config_cell_widths_field_from_query(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        })
        .or_else(|| {
            hyperlink_rule_variable.as_ref().and_then(|variable| {
                lua_static_window_effective_config_hyperlink_rule_field_from_query(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        })
        .or_else(|| {
            launch_menu_variable.as_ref().and_then(|variable| {
                lua_static_window_effective_config_launch_menu_field_from_query(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        })
        .or_else(|| {
            launch_menu_env_variable.as_ref().and_then(|variable| {
                lua_static_window_effective_config_launch_menu_env_field_from_query(
                    segment,
                    variable,
                    window_name,
                    static_source,
                    outer_static_source,
                )
            })
        }) {
            parts.push(NativeLuaWindowEffectiveConfigStatusPart::Field(field));
            has_dynamic_part = true;
        } else if let Some(text) = lua_static_string_value_from_expression(None, None, segment) {
            parts.push(NativeLuaWindowEffectiveConfigStatusPart::Static(text));
        } else {
            return None;
        }
    }

    has_dynamic_part.then_some(NativeLuaWindowStatusText::WindowEffectiveConfig { parts })
}

fn lua_static_window_effective_config_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_variable_from_statement(statement, window_name)
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_variable_from_statement(
    statement: &str,
    window_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    if lua_window_zero_arg_method_name_from_query(value, window_name)? != "effective_config" {
        return None;
    }
    Some(variable.to_owned())
}

fn lua_static_window_effective_config_resolved_palette_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_resolved_palette_variable_from_statement(
                statement,
                window_name,
            )
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_resolved_palette_variable_from_statement(
    statement: &str,
    window_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    if !lua_window_effective_config_resolved_palette_from_query(value, window_name)? {
        return None;
    }
    Some(variable.to_owned())
}

fn lua_static_window_effective_config_visual_bell_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_visual_bell_variable_from_statement(
                statement,
                window_name,
            )
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_visual_bell_variable_from_statement(
    statement: &str,
    window_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    if !lua_window_effective_config_visual_bell_from_query(value, window_name)? {
        return None;
    }
    Some(variable.to_owned())
}

fn lua_static_window_effective_config_environment_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_environment_variable_from_statement(
                statement,
                window_name,
            )
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_environment_variable_from_statement(
    statement: &str,
    window_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    if !lua_window_effective_config_environment_from_query(value, window_name)? {
        return None;
    }
    Some(variable.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaCellWidthsVariableReference {
    variable: String,
    index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaHyperlinkRuleVariableReference {
    variable: String,
    index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaLaunchMenuVariableReference {
    variable: String,
    index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaLaunchMenuEnvVariableReference {
    variable: String,
    index: usize,
}

fn lua_static_window_effective_config_cell_widths_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<NativeLuaCellWidthsVariableReference> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_cell_widths_variable_from_statement(
                statement,
                window_name,
            )
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_cell_widths_variable_from_statement(
    statement: &str,
    window_name: &str,
) -> Option<NativeLuaCellWidthsVariableReference> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let index = lua_window_effective_config_cell_widths_entry_from_query(value, window_name)?;
    Some(NativeLuaCellWidthsVariableReference {
        variable: variable.to_owned(),
        index,
    })
}

fn lua_static_window_effective_config_hyperlink_rule_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<NativeLuaHyperlinkRuleVariableReference> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_hyperlink_rule_variable_from_statement(
                statement,
                window_name,
                Some(LuaStaticSource {
                    source,
                    max_start: start,
                }),
            )
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_hyperlink_rule_variable_from_statement(
    statement: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaHyperlinkRuleVariableReference> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let index = lua_window_effective_config_hyperlink_rule_entry_from_query_with_static_source(
        value,
        window_name,
        static_source,
    )?;
    Some(NativeLuaHyperlinkRuleVariableReference {
        variable: variable.to_owned(),
        index,
    })
}

fn lua_static_window_effective_config_launch_menu_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<NativeLuaLaunchMenuVariableReference> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_launch_menu_variable_from_statement(
                statement,
                window_name,
                Some(LuaStaticSource {
                    source,
                    max_start: start,
                }),
            )
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_launch_menu_variable_from_statement(
    statement: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaLaunchMenuVariableReference> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let index = lua_window_effective_config_launch_menu_entry_from_query_with_static_source(
        value,
        window_name,
        static_source,
    )?;
    Some(NativeLuaLaunchMenuVariableReference {
        variable: variable.to_owned(),
        index,
    })
}

fn lua_static_window_effective_config_launch_menu_env_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<NativeLuaLaunchMenuEnvVariableReference> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_effective_config_launch_menu_env_variable_from_statement(
                statement,
                window_name,
                Some(LuaStaticSource {
                    source,
                    max_start: start,
                }),
            )
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_effective_config_launch_menu_env_variable_from_statement(
    statement: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaLaunchMenuEnvVariableReference> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let index = lua_window_effective_config_launch_menu_env_from_query_with_static_source(
        value,
        window_name,
        static_source,
    )?;
    Some(NativeLuaLaunchMenuEnvVariableReference {
        variable: variable.to_owned(),
        index,
    })
}

fn lua_window_effective_config_resolved_palette_from_query(
    value: &str,
    window_name: &str,
) -> Option<bool> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if field != "resolved_palette" {
        return None;
    }
    lua_trim_start_comments(rest.get(field.len()..)?)?
        .is_empty()
        .then_some(true)
}

fn lua_window_effective_config_visual_bell_from_query(
    value: &str,
    window_name: &str,
) -> Option<bool> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if field != "visual_bell" {
        return None;
    }
    lua_trim_start_comments(rest.get(field.len()..)?)?
        .is_empty()
        .then_some(true)
}

fn lua_window_effective_config_environment_from_query(
    value: &str,
    window_name: &str,
) -> Option<bool> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if field != "set_environment_variables" {
        return None;
    }
    lua_trim_start_comments(rest.get(field.len()..)?)?
        .is_empty()
        .then_some(true)
}

fn lua_window_effective_config_cell_widths_entry_from_query(
    value: &str,
    window_name: &str,
) -> Option<usize> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if field != "cell_widths" {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(field.len()..)?)?;
    let (index, rest) = lua_table_array_index_access_rest_from_query(rest)?;
    lua_trim_start_comments(rest)?.is_empty().then_some(index)
}

fn lua_window_effective_config_hyperlink_rule_entry_from_query_with_static_source(
    value: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<usize> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if field != "hyperlink_rules" {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(field.len()..)?)?;
    let (index, rest) =
        lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
    lua_trim_start_comments(rest)?.is_empty().then_some(index)
}

fn lua_window_effective_config_launch_menu_entry_from_query_with_static_source(
    value: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<usize> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if field != "launch_menu" {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(field.len()..)?)?;
    let (index, rest) =
        lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
    lua_trim_start_comments(rest)?.is_empty().then_some(index)
}

fn lua_window_effective_config_launch_menu_env_from_query_with_static_source(
    value: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<usize> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if field != "launch_menu" {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(field.len()..)?)?;
    let (index, rest) =
        lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let rest = lua_trim_start_comments(rest)?;
    let nested_field = lua_identifier_literal_from_query(rest)?;
    if nested_field != "set_environment_variables" {
        return None;
    }
    lua_trim_start_comments(rest.get(nested_field.len()..)?)?
        .is_empty()
        .then_some(index)
}

fn lua_static_window_effective_config_field_from_query_with_static_sources(
    value: &str,
    variable: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    if let Some((key, field_rest)) = lua_table_map_field_key_from_query_with_static_sources(
        static_source,
        outer_static_source,
        rest,
    ) {
        let quoted_key = lua_single_quoted_string_query_literal(&key);
        let synthetic = format!("{window_name}:effective_config()[{quoted_key}]{field_rest}");
        return lua_window_effective_config_field_from_query_with_static_sources(
            &synthetic,
            window_name,
            static_source,
            outer_static_source,
        );
    }
    let synthetic = format!("{window_name}:effective_config(){rest}");
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_single_quoted_string_query_literal(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

fn lua_static_window_effective_config_resolved_palette_field_from_query(
    value: &str,
    variable: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let synthetic = format!("{window_name}:effective_config().resolved_palette{rest}");
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_static_window_effective_config_visual_bell_field_from_query(
    value: &str,
    variable: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let synthetic = format!("{window_name}:effective_config().visual_bell{rest}");
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_static_window_effective_config_environment_field_from_query(
    value: &str,
    variable: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let synthetic = format!("{window_name}:effective_config().set_environment_variables{rest}");
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_static_window_effective_config_cell_widths_field_from_query(
    value: &str,
    variable: &NativeLuaCellWidthsVariableReference,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(&variable.variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let synthetic = format!(
        "{window_name}:effective_config().cell_widths[{}]{rest}",
        variable.index
    );
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_static_window_effective_config_hyperlink_rule_field_from_query(
    value: &str,
    variable: &NativeLuaHyperlinkRuleVariableReference,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(&variable.variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let synthetic = format!(
        "{window_name}:effective_config().hyperlink_rules[{}]{rest}",
        variable.index
    );
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_static_window_effective_config_launch_menu_field_from_query(
    value: &str,
    variable: &NativeLuaLaunchMenuVariableReference,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(&variable.variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let synthetic = format!(
        "{window_name}:effective_config().launch_menu[{}]{rest}",
        variable.index
    );
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_static_window_effective_config_launch_menu_env_field_from_query(
    value: &str,
    variable: &NativeLuaLaunchMenuEnvVariableReference,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(&variable.variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let synthetic = format!(
        "{window_name}:effective_config().launch_menu[{}].set_environment_variables{rest}",
        variable.index
    );
    lua_window_effective_config_field_from_query_with_static_sources(
        &synthetic,
        window_name,
        static_source,
        outer_static_source,
    )
}

fn lua_static_pane_dimensions_status_text_from_query(
    static_source: LuaStaticSource<'_>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let variable = lua_static_pane_dimensions_variable_before_offset(
        static_source.source,
        window_name,
        pane_name,
        static_source.max_start,
    )?;
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        if let Some(field) = lua_static_pane_dimensions_field_from_query(segment, &variable) {
            parts.push(NativeLuaPaneDimensionsStatusPart::Field(field));
            has_dynamic_part = true;
        } else if let Some(text) = lua_static_string_value_from_expression(None, None, segment) {
            parts.push(NativeLuaPaneDimensionsStatusPart::Static(text));
        } else {
            return None;
        }
    }

    has_dynamic_part.then_some(NativeLuaWindowStatusText::PaneDimensions { parts })
}

fn lua_static_pane_cursor_position_status_text_from_query(
    static_source: LuaStaticSource<'_>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let variable = lua_static_pane_cursor_position_variable_before_offset(
        static_source.source,
        window_name,
        pane_name,
        static_source.max_start,
    )?;
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        if let Some(field) = lua_static_pane_cursor_position_field_from_query(segment, &variable) {
            parts.push(NativeLuaPaneCursorPositionStatusPart::Field(field));
            has_dynamic_part = true;
        } else if let Some(text) = lua_static_string_value_from_expression(None, None, segment) {
            parts.push(NativeLuaPaneCursorPositionStatusPart::Static(text));
        } else {
            return None;
        }
    }

    has_dynamic_part.then_some(NativeLuaWindowStatusText::PaneCursorPosition { parts })
}

fn lua_static_pane_user_vars_status_text_from_query(
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let variable = lua_static_pane_user_vars_variable_before_offset(
        static_source.source,
        window_name,
        pane_name,
        static_source.max_start,
    );
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        if let Some((name, fallback)) = lua_static_pane_user_var_fallback_from_query(
            static_source,
            outer_static_source,
            segment,
            variable.as_deref(),
            window_name,
            pane_name,
        ) {
            parts.push(NativeLuaPaneUserVarsStatusPart::UserVar { name, fallback });
            has_dynamic_part = true;
        } else if let Some(name) = lua_static_pane_user_var_name_from_query(
            Some(static_source),
            outer_static_source,
            segment,
            variable.as_deref(),
            window_name,
            pane_name,
        ) {
            parts.push(NativeLuaPaneUserVarsStatusPart::UserVar {
                name,
                fallback: String::new(),
            });
            has_dynamic_part = true;
        } else if let Some((name, fallback)) = lua_static_pane_user_vars_local_value_from_query(
            static_source,
            outer_static_source,
            segment,
            variable.as_deref(),
            window_name,
            pane_name,
        ) {
            parts.push(NativeLuaPaneUserVarsStatusPart::UserVar { name, fallback });
            has_dynamic_part = true;
        } else if let Some(text) = lua_static_string_value_from_expression(None, None, segment) {
            parts.push(NativeLuaPaneUserVarsStatusPart::Static(text));
        } else {
            return None;
        }
    }

    has_dynamic_part.then_some(NativeLuaWindowStatusText::PaneUserVars { parts })
}

fn lua_static_pane_user_vars_local_value_from_query(
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    variable: Option<&str>,
    window_name: &str,
    pane_name: &str,
) -> Option<(String, String)> {
    let value = lua_tostring_argument_from_query(value).unwrap_or(value);
    let local_value = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
    )?;
    if let Some((name, fallback)) = lua_static_pane_user_var_fallback_from_query(
        static_source,
        outer_static_source,
        local_value,
        variable,
        window_name,
        pane_name,
    ) {
        return Some((name, fallback));
    }
    lua_static_pane_user_var_name_from_query(
        Some(static_source),
        outer_static_source,
        local_value,
        variable,
        window_name,
        pane_name,
    )
    .map(|name| (name, String::new()))
}

fn lua_static_pane_user_vars_variable_before_offset(
    source: &str,
    window_name: &str,
    pane_name: &str,
    max_start: usize,
) -> Option<String> {
    lua_static_pane_user_vars_variable_source_before_offset(
        source,
        window_name,
        pane_name,
        max_start,
    )
    .map(|(variable, _source)| variable)
}

fn lua_static_pane_user_vars_variable_source_before_offset(
    source: &str,
    window_name: &str,
    pane_name: &str,
    max_start: usize,
) -> Option<(String, NativeLuaUserVarChangedPaneUserVarSource)> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) = lua_static_pane_user_vars_variable_source_from_statement(
            statement,
            window_name,
            pane_name,
        ) {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_pane_user_vars_variable_source_from_statement(
    statement: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<(String, NativeLuaUserVarChangedPaneUserVarSource)> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let is_callback_pane =
        lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("get_user_vars");
    if is_callback_pane {
        return Some((
            variable.to_owned(),
            NativeLuaUserVarChangedPaneUserVarSource::EventPane,
        ));
    }
    let is_active_pane = lua_window_active_pane_zero_arg_method_name_from_query(value, window_name)
        == Some("get_user_vars");
    is_active_pane.then_some((
        variable.to_owned(),
        NativeLuaUserVarChangedPaneUserVarSource::ActivePane,
    ))
}

fn lua_static_pane_user_var_fallback_from_query(
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    variable: Option<&str>,
    window_name: &str,
    pane_name: &str,
) -> Option<(String, String)> {
    let value = lua_tostring_argument_from_query(value).unwrap_or(value);
    let (dynamic, fallback) = lua_dynamic_status_fallback_from_query(value)?;
    let name = lua_static_pane_user_var_name_from_query(
        Some(static_source),
        outer_static_source,
        dynamic,
        variable,
        window_name,
        pane_name,
    )?;
    let fallback = lua_static_string_value_from_expression(None, None, fallback)?;
    Some((name, fallback))
}

fn lua_static_pane_user_var_source_fallback_from_query(
    static_source: LuaStaticSource<'_>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    variable: Option<&(String, NativeLuaUserVarChangedPaneUserVarSource)>,
    window_name: &str,
    pane_name: &str,
) -> Option<(NativeLuaUserVarChangedPaneUserVarSource, String, String)> {
    let value = lua_tostring_argument_from_query(value).unwrap_or(value);
    let (dynamic, fallback) = lua_dynamic_status_fallback_from_query(value)?;
    let (source, name) = lua_static_pane_user_var_source_name_from_query(
        Some(static_source),
        outer_static_source,
        dynamic,
        variable,
        window_name,
        pane_name,
    )?;
    let fallback = lua_static_string_value_from_expression(None, None, fallback)?;
    Some((source, name, fallback))
}

fn lua_static_pane_user_var_source_name_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    variable: Option<&(String, NativeLuaUserVarChangedPaneUserVarSource)>,
    window_name: &str,
    pane_name: &str,
) -> Option<(NativeLuaUserVarChangedPaneUserVarSource, String)> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    if let Some((variable, source)) = variable
        && let Some(rest) = value.strip_prefix(variable)
    {
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            return None;
        }
        let name =
            lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)?;
        return Some((*source, name));
    }
    lua_static_direct_pane_user_var_source_name_from_query(
        static_source,
        outer_static_source,
        value,
        window_name,
        pane_name,
    )
}

fn lua_static_direct_pane_user_var_source_name_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<(NativeLuaUserVarChangedPaneUserVarSource, String)> {
    if let Some(rest) = lua_pane_get_user_vars_rest_from_query(value, pane_name) {
        return lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
            .map(|name| (NativeLuaUserVarChangedPaneUserVarSource::EventPane, name));
    }
    let rest = lua_window_active_pane_get_user_vars_rest_from_query(value, window_name)?;
    lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
        .map(|name| (NativeLuaUserVarChangedPaneUserVarSource::ActivePane, name))
}

fn lua_static_pane_user_var_name_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    variable: Option<&str>,
    window_name: &str,
    pane_name: &str,
) -> Option<String> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    if let Some(variable) = variable
        && let Some(rest) = value.strip_prefix(variable)
    {
        if rest.chars().next().is_some_and(is_lua_identifier_character) {
            return None;
        }
        return lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest);
    }
    lua_static_direct_pane_user_var_name_from_query(
        static_source,
        outer_static_source,
        value,
        window_name,
        pane_name,
    )
}

fn lua_static_direct_pane_user_var_name_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<String> {
    let rest = lua_pane_get_user_vars_rest_from_query(value, pane_name)
        .or_else(|| lua_window_active_pane_get_user_vars_rest_from_query(value, window_name))?;
    lua_static_pane_user_var_name_from_rest(static_source, outer_static_source, rest)
}

fn lua_pane_get_user_vars_rest_from_query<'a>(value: &'a str, pane_name: &str) -> Option<&'a str> {
    let rest = value.strip_prefix(pane_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "get_user_vars" || !lua_config_assignment_field_has_boundaries(rest, 0, method) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    lua_trim_start_comments(arguments)?
        .trim()
        .is_empty()
        .then_some(rest)
}

fn lua_window_active_pane_get_user_vars_rest_from_query<'a>(
    value: &'a str,
    window_name: &str,
) -> Option<&'a str> {
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let accessor = lua_identifier_literal_from_query(rest)?;
    if accessor != "active_pane" || !lua_config_assignment_field_has_boundaries(rest, 0, accessor) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(accessor.len()..)?)?.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "get_user_vars" || !lua_config_assignment_field_has_boundaries(rest, 0, method) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    lua_trim_start_comments(arguments)?
        .trim()
        .is_empty()
        .then_some(rest)
}

fn lua_static_pane_user_var_name_from_rest(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    rest: &str,
) -> Option<String> {
    let rest = lua_trim_start_comments(rest)?;
    if let Some(rest) = rest.strip_prefix('.') {
        let name = lua_identifier_literal_from_query(rest)?;
        if !lua_trim_start_comments(rest.get(name.len()..)?)?.is_empty() {
            return None;
        }
        return Some(name.to_owned());
    }
    let rest = rest.strip_prefix('[')?;
    let rest = lua_trim_start_comments(rest)?;
    let (name, rest) =
        lua_static_pane_user_var_bracket_key_from_query(static_source, outer_static_source, rest)?;
    let rest = lua_trim_start_comments(rest)?.strip_prefix(']')?;
    lua_trim_start_comments(rest)?.is_empty().then_some(name)
}

fn lua_static_pane_user_var_bracket_key_from_query<'a>(
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
    value: &'a str,
) -> Option<(String, &'a str)> {
    if let Some(key_literal) = lua_quoted_string_literal_from_query(value)
        .or_else(|| lua_long_bracket_literal_from_query(value))
    {
        return Some((
            parse_maybe_quoted_query_text(key_literal)?,
            value.get(key_literal.len()..)?,
        ));
    }

    let variable = lua_identifier_literal_from_query(value)?;
    let key =
        lua_static_string_value_from_expression(static_source, outer_static_source, variable)?;
    Some((key, value.get(variable.len()..)?))
}

fn lua_static_pane_cursor_position_variable_before_offset(
    source: &str,
    window_name: &str,
    pane_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) = lua_static_pane_cursor_position_variable_from_statement(
            statement,
            window_name,
            pane_name,
        ) {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_pane_cursor_position_variable_from_statement(
    statement: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let is_callback_pane =
        lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("get_cursor_position");
    let is_active_pane = lua_window_active_pane_zero_arg_method_name_from_query(value, window_name)
        == Some("get_cursor_position");
    if !is_callback_pane && !is_active_pane {
        return None;
    }
    Some(variable.to_owned())
}

fn lua_static_pane_cursor_position_field_from_query(
    value: &str,
    variable: &str,
) -> Option<NativeLuaPaneCursorPositionField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if !lua_trim_start_comments(rest.get(field.len()..)?)?.is_empty() {
        return None;
    }
    match field {
        "x" => Some(NativeLuaPaneCursorPositionField::X),
        "y" => Some(NativeLuaPaneCursorPositionField::Y),
        "shape" => Some(NativeLuaPaneCursorPositionField::Shape),
        "visibility" => Some(NativeLuaPaneCursorPositionField::Visibility),
        _ => None,
    }
}

fn lua_static_pane_dimensions_variable_before_offset(
    source: &str,
    window_name: &str,
    pane_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_pane_dimensions_variable_from_statement(statement, window_name, pane_name)
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_pane_dimensions_variable_from_statement(
    statement: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let is_callback_pane =
        lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("get_dimensions");
    let is_active_pane = lua_window_active_pane_zero_arg_method_name_from_query(value, window_name)
        == Some("get_dimensions");
    if !is_callback_pane && !is_active_pane {
        return None;
    }
    Some(variable.to_owned())
}

fn lua_static_pane_dimensions_field_from_query(
    value: &str,
    variable: &str,
) -> Option<NativeLuaPaneDimensionsField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if !lua_trim_start_comments(rest.get(field.len()..)?)?.is_empty() {
        return None;
    }
    match field {
        "cols" => Some(NativeLuaPaneDimensionsField::Cols),
        "viewport_rows" => Some(NativeLuaPaneDimensionsField::ViewportRows),
        "scrollback_rows" => Some(NativeLuaPaneDimensionsField::ScrollbackRows),
        "physical_top" => Some(NativeLuaPaneDimensionsField::PhysicalTop),
        "scrollback_top" => Some(NativeLuaPaneDimensionsField::ScrollbackTop),
        _ => None,
    }
}

fn lua_static_window_dimensions_variable_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_window_dimensions_variable_from_statement(statement, window_name)
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_window_dimensions_variable_from_statement(
    statement: &str,
    window_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    if lua_window_zero_arg_method_name_from_query(value, window_name)? != "get_dimensions" {
        return None;
    }
    Some(variable.to_owned())
}

fn lua_static_window_dimensions_field_from_query(
    value: &str,
    variable: &str,
) -> Option<NativeLuaWindowDimensionsField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let field = lua_identifier_literal_from_query(rest)?;
    if !lua_trim_start_comments(rest.get(field.len()..)?)?.is_empty() {
        return None;
    }
    match field {
        "pixel_width" => Some(NativeLuaWindowDimensionsField::PixelWidth),
        "pixel_height" => Some(NativeLuaWindowDimensionsField::PixelHeight),
        "dpi" => Some(NativeLuaWindowDimensionsField::Dpi),
        "is_full_screen" => Some(NativeLuaWindowDimensionsField::IsFullScreen),
        _ => None,
    }
}

fn lua_static_keyboard_modifiers_status_text_from_query(
    static_source: LuaStaticSource<'_>,
    window_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let (modifiers_variable, leds_variable) =
        lua_static_window_keyboard_modifiers_variables_before_offset(
            static_source.source,
            window_name,
            static_source.max_start,
        )?;
    let mut parts = Vec::new();
    let mut has_dynamic_part = false;

    for segment in split_lua_string_concat_segments(value)? {
        let segment = segment.trim();
        if segment == modifiers_variable {
            parts.push(NativeLuaKeyboardModifiersStatusPart::Modifiers);
            has_dynamic_part = true;
        } else if segment == leds_variable {
            parts.push(NativeLuaKeyboardModifiersStatusPart::Leds);
            has_dynamic_part = true;
        } else if let Some(text) = lua_static_string_value_from_expression(None, None, segment) {
            parts.push(NativeLuaKeyboardModifiersStatusPart::Static(text));
        } else {
            return None;
        }
    }

    has_dynamic_part.then_some(NativeLuaWindowStatusText::KeyboardModifiers { parts })
}

fn lua_static_window_keyboard_modifiers_variables_before_offset(
    source: &str,
    window_name: &str,
    max_start: usize,
) -> Option<(String, String)> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variables) =
            lua_static_window_keyboard_modifiers_variables_from_statement(statement, window_name)
        {
            selected = Some(variables);
        }
    }

    selected
}

fn lua_static_window_keyboard_modifiers_variables_from_statement(
    statement: &str,
    window_name: &str,
) -> Option<(String, String)> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let modifiers_variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(modifiers_variable.len()..)?)?;
    let rest = rest.strip_prefix(',')?;
    let rest = lua_trim_start_comments(rest)?;
    let leds_variable = lua_identifier_literal_from_query(rest)?;
    if modifiers_variable == leds_variable {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(leds_variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    if lua_window_zero_arg_method_name_from_query(value, window_name)? != "keyboard_modifiers" {
        return None;
    }
    Some((modifiers_variable.to_owned(), leds_variable.to_owned()))
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn lua_static_window_status_variable_text_from_query(
    static_source: LuaStaticSource<'_>,
    window_name: &str,
    pane_name: &str,
    value: &str,
) -> Option<NativeLuaWindowStatusText> {
    let (variable, fallback_text) = lua_window_status_variable_fallback_from_query(value)?;
    let assignment = lua_static_expression_variable_assignment_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )?;
    if let Some(mut status) = lua_window_status_method_text_from_query(assignment, window_name) {
        match &mut status {
            NativeLuaWindowStatusText::ActiveKeyTable { prefix, fallback }
            | NativeLuaWindowStatusText::CompositionStatus { prefix, fallback } => {
                if let Some(parsed_prefix) = lua_static_window_status_variable_prefix_before_offset(
                    static_source.source,
                    variable,
                    static_source.max_start,
                ) {
                    *prefix = parsed_prefix;
                }
                if let Some(parsed_fallback) = fallback_text {
                    *fallback = parsed_fallback;
                }
            }
            NativeLuaWindowStatusText::Static(_)
            | NativeLuaWindowStatusText::ActiveWorkspace
            | NativeLuaWindowStatusText::WindowId { .. }
            | NativeLuaWindowStatusText::WindowPane { .. }
            | NativeLuaWindowStatusText::Leader { .. }
            | NativeLuaWindowStatusText::Focus { .. }
            | NativeLuaWindowStatusText::PaneAltScreen { .. }
            | NativeLuaWindowStatusText::PaneHasUnseenOutput { .. }
            | NativeLuaWindowStatusText::WindowDimensions { .. }
            | NativeLuaWindowStatusText::WindowEffectiveConfig { .. }
            | NativeLuaWindowStatusText::PaneDimensions { .. }
            | NativeLuaWindowStatusText::PaneCursorPosition { .. }
            | NativeLuaWindowStatusText::PaneUserVars { .. }
            | NativeLuaWindowStatusText::PaneProgress { .. }
            | NativeLuaWindowStatusText::KeyboardModifiers { .. } => {}
        }
        return Some(status);
    }

    let assignment = lua_tostring_argument_from_query(assignment).unwrap_or(assignment);
    if let Some(part) = lua_static_window_and_pane_status_part_receiver_alias_from_query(
        static_source,
        assignment,
        window_name,
    )
    .or_else(|| {
        lua_static_window_and_pane_status_part_from_query(assignment, window_name, pane_name)
    }) {
        return Some(NativeLuaWindowStatusText::WindowPane { parts: vec![part] });
    }

    let inactive = lua_static_string_value_from_expression(None, None, assignment)?;
    if let Some((focused, unfocused)) =
        lua_static_window_status_variable_bool_method_text_before_offset(
            static_source.source,
            variable,
            window_name,
            "is_focused",
            static_source.max_start,
        )
    {
        return Some(NativeLuaWindowStatusText::Focus {
            focused,
            unfocused: fallback_text
                .or(unfocused)
                .unwrap_or_else(|| inactive.clone()),
        });
    }
    if let Some((active, parsed_inactive)) =
        lua_static_window_status_variable_bool_method_text_before_offset(
            static_source.source,
            variable,
            pane_name,
            "is_alt_screen_active",
            static_source.max_start,
        )
        .or_else(|| {
            lua_static_window_status_variable_active_pane_bool_method_text_before_offset(
                static_source.source,
                variable,
                window_name,
                "is_alt_screen_active",
                static_source.max_start,
            )
        })
    {
        return Some(NativeLuaWindowStatusText::PaneAltScreen {
            active,
            inactive: fallback_text
                .or(parsed_inactive)
                .unwrap_or_else(|| inactive.clone()),
        });
    }
    if let Some((unseen, parsed_seen)) =
        lua_static_window_status_variable_bool_method_text_before_offset(
            static_source.source,
            variable,
            pane_name,
            "has_unseen_output",
            static_source.max_start,
        )
        .or_else(|| {
            lua_static_window_status_variable_active_pane_bool_method_text_before_offset(
                static_source.source,
                variable,
                window_name,
                "has_unseen_output",
                static_source.max_start,
            )
        })
    {
        return Some(NativeLuaWindowStatusText::PaneHasUnseenOutput {
            unseen,
            seen: fallback_text
                .or(parsed_seen)
                .unwrap_or_else(|| inactive.clone()),
        });
    }
    if let Some(status) = lua_static_pane_progress_status_variable_text_before_offset(
        static_source.source,
        variable,
        window_name,
        pane_name,
        fallback_text.as_deref().unwrap_or(&inactive),
        static_source.max_start,
    ) {
        return Some(status);
    }
    let active = lua_static_window_status_variable_leader_active_text_before_offset(
        static_source.source,
        variable,
        window_name,
        static_source.max_start,
    )?;
    Some(NativeLuaWindowStatusText::Leader {
        active,
        inactive: fallback_text.unwrap_or(inactive),
    })
}

fn lua_static_pane_progress_status_variable_text_before_offset(
    source: &str,
    status_variable: &str,
    window_name: &str,
    pane_name: &str,
    none: &str,
    max_start: usize,
) -> Option<NativeLuaWindowStatusText> {
    let progress_variable =
        lua_static_pane_progress_variable_before_offset(source, window_name, pane_name, max_start)?;
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(status) = lua_static_pane_progress_status_variable_text_from_statement(
            statement,
            status_variable,
            &progress_variable,
            none,
        ) {
            selected = Some(status);
        }
    }

    selected
}

fn lua_static_pane_progress_status_variable_text_from_statement(
    statement: &str,
    status_variable: &str,
    progress_variable: &str,
    none: &str,
) -> Option<NativeLuaWindowStatusText> {
    let (branches, else_body, _) =
        lua_static_if_condition_and_body_branches_and_else_from_statement(statement)?;
    if else_body.is_some() {
        return None;
    }

    let mut percentage_prefix = None;
    let mut error_prefix = None;
    let mut indeterminate = None;

    for (condition, body) in branches {
        if lua_static_pane_progress_field_not_nil_condition_from_query(
            condition,
            progress_variable,
            "Percentage",
        ) {
            percentage_prefix = Some(lua_static_pane_progress_field_assignment_prefix_from_body(
                body,
                status_variable,
                progress_variable,
                "Percentage",
            )?);
        } else if lua_static_pane_progress_field_not_nil_condition_from_query(
            condition,
            progress_variable,
            "Error",
        ) {
            error_prefix = Some(lua_static_pane_progress_field_assignment_prefix_from_body(
                body,
                status_variable,
                progress_variable,
                "Error",
            )?);
        } else if lua_static_pane_progress_string_condition_from_query(
            condition,
            progress_variable,
            "Indeterminate",
        ) {
            indeterminate = Some(lua_static_pane_progress_state_assignment_text_from_body(
                body,
                status_variable,
                progress_variable,
                "Indeterminate",
            )?);
        } else {
            return None;
        }
    }

    Some(NativeLuaWindowStatusText::PaneProgress {
        none: none.to_owned(),
        percentage_prefix: percentage_prefix?,
        error_prefix: error_prefix?,
        indeterminate: indeterminate.unwrap_or_else(|| "Indeterminate".to_owned()),
    })
}

fn lua_static_pane_progress_variable_before_offset(
    source: &str,
    window_name: &str,
    pane_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(variable) =
            lua_static_pane_progress_variable_from_statement(statement, window_name, pane_name)
        {
            selected = Some(variable);
        }
    }

    selected
}

fn lua_static_pane_progress_variable_from_statement(
    statement: &str,
    window_name: &str,
    pane_name: &str,
) -> Option<String> {
    let statement = lua_trim_start_comments(statement)?;
    let rest = if lua_source_keyword_at(statement, 0, "local") {
        lua_trim_start_comments(statement.get("local".len()..)?)?
    } else {
        statement
    };
    let variable = lua_identifier_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(variable.len()..)?)?;
    let rest = rest.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let is_callback_pane =
        lua_window_zero_arg_method_name_from_query(value, pane_name) == Some("get_progress");
    let is_active_pane = lua_window_active_pane_zero_arg_method_name_from_query(value, window_name)
        == Some("get_progress");
    if !is_callback_pane && !is_active_pane {
        return None;
    }
    Some(variable.to_owned())
}

fn lua_static_pane_progress_field_not_nil_condition_from_query(
    condition: &str,
    progress_variable: &str,
    field: &str,
) -> bool {
    let Some(rest) =
        lua_static_pane_progress_field_rest_from_query(condition, progress_variable, field)
    else {
        return false;
    };
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("~=") else {
        return false;
    };
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    lua_source_keyword_at(rest, 0, "nil")
        && lua_static_identifier_value_rest_is_statement_end(
            rest.get("nil".len()..).unwrap_or_default(),
        )
}

fn lua_static_pane_progress_string_condition_from_query(
    condition: &str,
    progress_variable: &str,
    expected: &str,
) -> bool {
    let Some(rest) = lua_trim_start_comments(condition) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(progress_variable) else {
        return false;
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return false;
    }
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("==") else {
        return false;
    };
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    lua_static_string_value_from_expression(None, None, rest).is_some_and(|value| value == expected)
}

fn lua_static_pane_progress_field_rest_from_query<'a>(
    value: &'a str,
    progress_variable: &str,
    field: &str,
) -> Option<&'a str> {
    let value = lua_trim_start_comments(value)?.trim();
    let rest = value.strip_prefix(progress_variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('.')?;
    let parsed_field = lua_identifier_literal_from_query(rest)?;
    if parsed_field != field {
        return None;
    }
    rest.get(parsed_field.len()..)
}

fn lua_static_pane_progress_field_assignment_prefix_from_body(
    body: &str,
    status_variable: &str,
    progress_variable: &str,
    field: &str,
) -> Option<String> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(prefix) = lua_static_pane_progress_field_assignment_prefix_from_statement(
            statement,
            status_variable,
            progress_variable,
            field,
        ) {
            selected = Some(prefix);
        }
    }
    selected
}

fn lua_static_pane_progress_field_assignment_prefix_from_statement(
    statement: &str,
    status_variable: &str,
    progress_variable: &str,
    field: &str,
) -> Option<String> {
    let rest = statement.strip_prefix(status_variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let segments = split_lua_string_concat_segments(value)?;
    let [prefix, dynamic] = segments.as_slice() else {
        return None;
    };
    if !lua_static_pane_progress_field_expression_from_query(dynamic, progress_variable, field) {
        return None;
    }
    lua_static_string_value_from_expression(None, None, prefix)
}

fn lua_static_pane_progress_field_expression_from_query(
    value: &str,
    progress_variable: &str,
    field: &str,
) -> bool {
    let Some(rest) =
        lua_static_pane_progress_field_rest_from_query(value, progress_variable, field)
    else {
        return false;
    };
    lua_static_identifier_value_rest_is_statement_end(rest)
}

fn lua_static_pane_progress_state_assignment_text_from_body(
    body: &str,
    status_variable: &str,
    progress_variable: &str,
    default_text: &str,
) -> Option<String> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(text) = lua_static_pane_progress_state_assignment_text_from_statement(
            statement,
            status_variable,
            progress_variable,
            default_text,
        ) {
            selected = Some(text);
        }
    }
    selected
}

fn lua_static_pane_progress_state_assignment_text_from_statement(
    statement: &str,
    status_variable: &str,
    progress_variable: &str,
    default_text: &str,
) -> Option<String> {
    let rest = statement.strip_prefix(status_variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let value = lua_trim_start_comments(value)?;
    if value.strip_prefix(progress_variable).is_some_and(|rest| {
        !rest.chars().next().is_some_and(is_lua_identifier_character)
            && lua_static_identifier_value_rest_is_statement_end(rest)
    }) {
        return Some(default_text.to_owned());
    }
    lua_static_string_value_from_expression(None, None, value)
}

fn lua_static_window_status_variable_bool_method_text_before_offset(
    source: &str,
    variable: &str,
    window_name: &str,
    method: &str,
    max_start: usize,
) -> Option<(String, Option<String>)> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(value) = lua_static_window_status_variable_bool_method_text_from_statement(
            statement,
            variable,
            window_name,
            method,
        ) {
            selected = Some(value);
        }
    }

    selected
}

fn lua_static_window_status_variable_bool_method_text_from_statement(
    statement: &str,
    variable: &str,
    window_name: &str,
    method: &str,
) -> Option<(String, Option<String>)> {
    let (branches, else_body, _) =
        lua_static_if_condition_and_body_branches_and_else_from_statement(statement)?;
    let [(condition, body)] = branches.as_slice() else {
        return None;
    };
    if lua_window_zero_arg_method_name_from_query(condition, window_name)? != method {
        return None;
    }

    let active = lua_static_window_status_variable_static_assignment_from_body(body, variable)?;
    let inactive = else_body.and_then(|body| {
        lua_static_window_status_variable_static_assignment_from_body(body, variable)
    });
    Some((active, inactive))
}

fn lua_static_window_status_variable_active_pane_bool_method_text_before_offset(
    source: &str,
    variable: &str,
    window_name: &str,
    method: &str,
    max_start: usize,
) -> Option<(String, Option<String>)> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(value) =
            lua_static_window_status_variable_active_pane_bool_method_text_from_statement(
                statement,
                variable,
                window_name,
                method,
            )
        {
            selected = Some(value);
        }
    }

    selected
}

fn lua_static_window_status_variable_active_pane_bool_method_text_from_statement(
    statement: &str,
    variable: &str,
    window_name: &str,
    method: &str,
) -> Option<(String, Option<String>)> {
    let (branches, else_body, _) =
        lua_static_if_condition_and_body_branches_and_else_from_statement(statement)?;
    let [(condition, body)] = branches.as_slice() else {
        return None;
    };
    if lua_window_active_pane_zero_arg_method_name_from_query(condition, window_name)? != method {
        return None;
    }

    let active = lua_static_window_status_variable_static_assignment_from_body(body, variable)?;
    let inactive = else_body.and_then(|body| {
        lua_static_window_status_variable_static_assignment_from_body(body, variable)
    });
    Some((active, inactive))
}

fn lua_static_window_status_variable_leader_active_text_before_offset(
    source: &str,
    variable: &str,
    window_name: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(active) = lua_static_window_status_variable_leader_active_text_from_statement(
            statement,
            variable,
            window_name,
        ) {
            selected = Some(active);
        }
    }

    selected
}

fn lua_static_window_status_variable_static_assignment_from_body(
    body: &str,
    variable: &str,
) -> Option<String> {
    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(value) =
            lua_static_window_status_variable_static_assignment_from_statement(statement, variable)
        {
            selected = Some(value);
        }
    }
    selected
}

fn lua_static_window_status_variable_leader_active_text_from_statement(
    statement: &str,
    variable: &str,
    window_name: &str,
) -> Option<String> {
    let (branches, _) = lua_static_if_condition_and_body_branches_from_statement(statement)?;
    let [(condition, body)] = branches.as_slice() else {
        return None;
    };
    if lua_window_zero_arg_method_name_from_query(condition, window_name)? != "leader_is_active" {
        return None;
    }

    lua_static_window_status_variable_static_assignment_from_body(body, variable)
}

fn lua_window_status_variable_fallback_from_query(value: &str) -> Option<(&str, Option<String>)> {
    let value = lua_trim_start_comments(value)?;
    let variable = lua_identifier_literal_from_query(value)?;
    let rest = lua_trim_start_comments(value.get(variable.len()..)?)?;
    if rest.is_empty() {
        return Some((variable, None));
    }
    if !lua_source_keyword_at(rest, 0, "or") {
        return None;
    }
    let fallback = lua_trim_start_comments(rest.get("or".len()..)?)?;
    let fallback = lua_static_string_value_from_expression(None, None, fallback)?;
    Some((variable, Some(fallback)))
}

fn lua_static_window_status_variable_prefix_before_offset(
    source: &str,
    variable: &str,
    max_start: usize,
) -> Option<String> {
    let mut selected = None;

    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        let statement = lua_trim_start_comments(source.get(start..)?)?;
        if let Some(prefix) =
            lua_static_window_status_variable_prefix_from_statement(statement, variable)
        {
            selected = Some(prefix);
        }
    }

    selected
}

fn lua_static_window_status_variable_prefix_from_statement(
    statement: &str,
    variable: &str,
) -> Option<String> {
    let (branches, _) = lua_static_if_condition_and_body_branches_from_statement(statement)?;
    let [(condition, body)] = branches.as_slice() else {
        return None;
    };
    if *condition != variable {
        return None;
    }

    let mut selected = None;
    for start in lua_top_level_statement_start_indices_before_offset(body, body.len())? {
        let statement = lua_trim_start_comments(body.get(start..)?)?;
        if let Some(prefix) =
            lua_static_window_status_variable_prefix_assignment_from_statement(statement, variable)
        {
            selected = Some(prefix);
        }
    }
    selected
}

fn lua_static_window_status_variable_prefix_assignment_from_statement(
    statement: &str,
    variable: &str,
) -> Option<String> {
    let rest = statement.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    let segments = split_lua_string_concat_segments(value)?;
    let [prefix, dynamic] = segments.as_slice() else {
        return None;
    };
    if dynamic.trim() != variable {
        return None;
    }
    lua_static_string_value_from_expression(None, None, prefix)
}

fn lua_static_window_status_variable_static_assignment_from_statement(
    statement: &str,
    variable: &str,
) -> Option<String> {
    let rest = statement.strip_prefix(variable)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix('=')?;
    let value = lua_top_level_statement_value_from_query(rest)?;
    lua_static_string_value_from_expression(None, None, value)
}

fn lua_window_status_method_text_from_query(
    value: &str,
    window_name: &str,
) -> Option<NativeLuaWindowStatusText> {
    let method = lua_window_zero_arg_method_name_from_query(value, window_name)?;

    match method {
        "active_workspace" => Some(NativeLuaWindowStatusText::ActiveWorkspace),
        "window_id" => Some(NativeLuaWindowStatusText::WindowId {
            prefix: String::new(),
            suffix: String::new(),
        }),
        "active_key_table" => Some(NativeLuaWindowStatusText::ActiveKeyTable {
            prefix: String::new(),
            fallback: String::new(),
        }),
        "composition_status" => Some(NativeLuaWindowStatusText::CompositionStatus {
            prefix: String::new(),
            fallback: String::new(),
        }),
        _ => None,
    }
}

fn lua_window_effective_config_field_from_query_with_static_sources(
    value: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
    outer_static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    lua_window_effective_config_field_from_query_with_static_source(
        value,
        window_name,
        static_source,
    )
    .or_else(|| {
        lua_window_effective_config_field_from_query_with_static_source(
            value,
            window_name,
            outer_static_source,
        )
    })
}

fn lua_window_effective_config_field_from_query_with_static_source(
    value: &str,
    window_name: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    let value = lua_trim_start_comments(value)?.trim();
    let value = if value.starts_with("tostring")
        && lua_config_assignment_field_has_boundaries(value, 0, "tostring")
    {
        let rest = lua_trim_start_comments(value.get("tostring".len()..)?)?;
        let rest = rest.strip_prefix('(')?;
        let (argument, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
        if !lua_trim_start_comments(rest)?.is_empty() {
            return None;
        }
        lua_trim_start_comments(argument)?.trim()
    } else {
        value
    };
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if method != "effective_config" || !lua_config_assignment_field_has_boundaries(rest, 0, method)
    {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }
    let (field, rest) = lua_table_map_field_key_from_query_with_static_source(
        static_source,
        lua_trim_start_comments(rest)?,
    )?;
    let rest = lua_trim_start_comments(rest)?;
    if let Some(field) = lua_structured_effective_config_field_part1(&field, rest, static_source) {
        return Some(field);
    }
    if let Some(field) = lua_structured_effective_config_field_part2(&field, rest, static_source) {
        return Some(field);
    }
    if !rest.is_empty() {
        return None;
    }
    lua_scalar_effective_config_field_part1(&field)
        .or_else(|| lua_scalar_effective_config_field_part2(&field))
}


#[expect(
    clippy::too_many_lines,
    reason = "the compatibility evaluator remains ordered to preserve Lua precedence"
)]
fn lua_structured_effective_config_field_part1(
    field: &str,
    rest: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    if field == "launch_menu" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if nested_field == "label" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::LaunchMenu(
                index,
                NativeLuaLaunchMenuField::Label,
            ));
        }
        if nested_field == "cwd" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::LaunchMenu(
                index,
                NativeLuaLaunchMenuField::Cwd,
            ));
        }
        if nested_field == "domain" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::LaunchMenu(
                index,
                NativeLuaLaunchMenuField::Domain,
            ));
        }
        if nested_field == "args" {
            let (arg_index, rest) =
                lua_table_array_index_access_rest_from_query_with_static_source(
                    static_source,
                    nested_rest,
                )?;
            if lua_trim_start_comments(rest)?.is_empty() {
                return Some(NativeLuaWindowEffectiveConfigField::LaunchMenu(
                    index,
                    NativeLuaLaunchMenuField::Arg(arg_index),
                ));
            }
        }
        if nested_field == "set_environment_variables" {
            let (name, rest) =
                lua_table_map_field_key_from_query_with_static_source(static_source, nested_rest)?;
            if lua_trim_start_comments(rest)?.is_empty() {
                return Some(NativeLuaWindowEffectiveConfigField::LaunchMenu(
                    index,
                    NativeLuaLaunchMenuField::SetEnvironmentVariable(name),
                ));
            }
        }
        return None;
    }
    if field == "tiling_desktop_environments" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::TilingDesktopEnvironment(index));
        }
        return None;
    }
    if field == "mux_env_remove" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::MuxEnvRemove(index));
        }
        return None;
    }
    if field == "daemon_options" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        if !lua_trim_start_comments(nested_rest)?.is_empty() {
            return None;
        }
        let daemon_field = match nested_field.as_str() {
            "pid_file" => NativeLuaDaemonOptionsField::PidFile,
            "stdout" => NativeLuaDaemonOptionsField::Stdout,
            "stderr" => NativeLuaDaemonOptionsField::Stderr,
            _ => return None,
        };
        return Some(NativeLuaWindowEffectiveConfigField::DaemonOption(
            daemon_field,
        ));
    }
    if field == "harfbuzz_features" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::HarfbuzzFeature(index));
        }
        return None;
    }
    if field == "font_dirs" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::FontDir(index));
        }
        return None;
    }
    if field == "cell_widths" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if !nested_rest.is_empty() {
            return None;
        }
        let cell_width_field = match nested_field.as_str() {
            "first" => NativeLuaCellWidthOverrideField::First,
            "last" => NativeLuaCellWidthOverrideField::Last,
            "width" => NativeLuaCellWidthOverrideField::Width,
            _ => return None,
        };
        return Some(NativeLuaWindowEffectiveConfigField::CellWidths(
            index,
            cell_width_field,
        ));
    }
    if field == "quick_select_patterns" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::QuickSelectPattern(
                index,
            ));
        }
        return None;
    }
    if field == "hyperlink_rules" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        if !lua_trim_start_comments(nested_rest)?.is_empty() {
            return None;
        }
        let field = match nested_field.as_str() {
            "regex" => NativeLuaHyperlinkRuleField::Regex,
            "format" => NativeLuaHyperlinkRuleField::Format,
            "highlight" => NativeLuaHyperlinkRuleField::Highlight,
            _ => return None,
        };
        return Some(NativeLuaWindowEffectiveConfigField::HyperlinkRule(
            index, field,
        ));
    }
    if field == "color_scheme_dirs" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::ColorSchemeDir(index));
        }
        return None;
    }
    if field == "clean_exit_codes" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::CleanExitCode(index));
        }
        return None;
    }
    if field == "set_environment_variables" {
        let (name, rest) =
            lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::SetEnvironmentVariable(
                name,
            ));
        }
        return None;
    }
    if field == "default_prog" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::DefaultProg(index));
        }
        return None;
    }
    if field == "default_gui_startup_args" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::DefaultGuiStartupArg(
                index,
            ));
        }
        return None;
    }
    if field == "skip_close_confirmation_for_processes_named" {
        let (index, rest) =
            lua_table_array_index_access_rest_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::SkipCloseConfirmationProcess(index));
        }
        return None;
    }
    if field == "integrated_title_buttons" {
        let (index, rest) = lua_table_array_index_access_rest_from_query(rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::IntegratedTitleButton(
                index,
            ));
        }
        return None;
    }
    if field == "window_padding" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if nested_field == "left" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::WindowPaddingLeft);
        }
        if nested_field == "right" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::WindowPaddingRight);
        }
        if nested_field == "top" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::WindowPaddingTop);
        }
        if nested_field == "bottom" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::WindowPaddingBottom);
        }
        return None;
    }
    if field == "window_content_alignment" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if nested_field == "horizontal" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::WindowContentAlignmentHorizontal);
        }
        if nested_field == "vertical" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::WindowContentAlignmentVertical);
        }
        return None;
    }
    None
}

#[expect(
    clippy::too_many_lines,
    reason = "the compatibility evaluator remains ordered to preserve Lua precedence"
)]
fn lua_structured_effective_config_field_part2(
    field: &str,
    rest: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    if field == "webgpu_preferred_adapter" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if !nested_rest.is_empty() {
            return None;
        }
        let adapter_field = match nested_field.as_str() {
            "backend" => NativeLuaWebGpuPreferredAdapterField::Backend,
            "device" => NativeLuaWebGpuPreferredAdapterField::Device,
            "device_type" => NativeLuaWebGpuPreferredAdapterField::DeviceType,
            "driver" => NativeLuaWebGpuPreferredAdapterField::Driver,
            "driver_info" => NativeLuaWebGpuPreferredAdapterField::DriverInfo,
            "name" => NativeLuaWebGpuPreferredAdapterField::Name,
            "vendor" => NativeLuaWebGpuPreferredAdapterField::Vendor,
            _ => return None,
        };
        return Some(NativeLuaWindowEffectiveConfigField::WebGpuPreferredAdapter(
            adapter_field,
        ));
    }
    if field == "visual_bell" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if !nested_rest.is_empty() {
            return None;
        }
        let visual_bell_field = match nested_field.as_str() {
            "fade_in_duration_ms" => NativeLuaVisualBellField::FadeInDurationMs,
            "fade_out_duration_ms" => NativeLuaVisualBellField::FadeOutDurationMs,
            "fade_in_function" => NativeLuaVisualBellField::FadeInFunction,
            "fade_out_function" => NativeLuaVisualBellField::FadeOutFunction,
            "target" => NativeLuaVisualBellField::Target,
            _ => return None,
        };
        return Some(NativeLuaWindowEffectiveConfigField::VisualBell(
            visual_bell_field,
        ));
    }
    if field == "dpi_by_screen" {
        let (name, rest) =
            lua_table_map_field_key_from_query_with_static_source(static_source, rest)?;
        if lua_trim_start_comments(rest)?.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::DpiByScreen(name));
        }
        return None;
    }
    if field == "resolved_palette" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if nested_field == "ansi" {
            let (index, rest) = lua_table_array_index_access_rest_from_query_with_static_source(
                static_source,
                nested_rest,
            )?;
            if lua_trim_start_comments(rest)?.is_empty() {
                return Some(NativeLuaWindowEffectiveConfigField::ResolvedPalette(
                    NativeLuaResolvedPaletteField::Ansi(index),
                ));
            }
            return None;
        }
        if nested_field == "brights" {
            let (index, rest) = lua_table_array_index_access_rest_from_query_with_static_source(
                static_source,
                nested_rest,
            )?;
            if lua_trim_start_comments(rest)?.is_empty() {
                return Some(NativeLuaWindowEffectiveConfigField::ResolvedPalette(
                    NativeLuaResolvedPaletteField::Bright(index),
                ));
            }
            return None;
        }
        if nested_field == "indexed" {
            let (index, rest) = lua_table_array_index_access_rest_from_query_with_static_source(
                static_source,
                nested_rest,
            )?;
            if lua_trim_start_comments(rest)?.is_empty() {
                return Some(NativeLuaWindowEffectiveConfigField::ResolvedPalette(
                    NativeLuaResolvedPaletteField::Indexed(index),
                ));
            }
            return None;
        }
        if !nested_rest.is_empty() {
            return None;
        }
        let palette_field = match nested_field.as_str() {
            "foreground" => NativeLuaResolvedPaletteField::Foreground,
            "background" => NativeLuaResolvedPaletteField::Background,
            "cursor_bg" => NativeLuaResolvedPaletteField::CursorBg,
            "cursor_fg" => NativeLuaResolvedPaletteField::CursorFg,
            "cursor_border" => NativeLuaResolvedPaletteField::CursorBorder,
            "selection_fg" => NativeLuaResolvedPaletteField::SelectionFg,
            "selection_bg" => NativeLuaResolvedPaletteField::SelectionBg,
            "compose_cursor" => NativeLuaResolvedPaletteField::ComposeCursor,
            "visual_bell" => NativeLuaResolvedPaletteField::VisualBell,
            _ => return None,
        };
        return Some(NativeLuaWindowEffectiveConfigField::ResolvedPalette(
            palette_field,
        ));
    }
    if field == "foreground_text_hsb" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if nested_field == "hue" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::ForegroundTextHsbHue);
        }
        if nested_field == "saturation" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::ForegroundTextHsbSaturation);
        }
        if nested_field == "brightness" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::ForegroundTextHsbBrightness);
        }
        return None;
    }
    if field == "inactive_pane_hsb" {
        let (nested_field, nested_rest) = lua_table_map_field_key_from_query_with_static_source(
            static_source,
            lua_trim_start_comments(rest)?,
        )?;
        let nested_rest = lua_trim_start_comments(nested_rest)?;
        if nested_field == "hue" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::InactivePaneHsbHue);
        }
        if nested_field == "saturation" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::InactivePaneHsbSaturation);
        }
        if nested_field == "brightness" && nested_rest.is_empty() {
            return Some(NativeLuaWindowEffectiveConfigField::InactivePaneHsbBrightness);
        }
        return None;
    }
    None
}

#[expect(
    clippy::too_many_lines,
    reason = "the compatibility evaluator remains ordered to preserve Lua precedence"
)]
fn lua_scalar_effective_config_field_part1(
    field: &str,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    match field {
       "font_size" => Some(NativeLuaWindowEffectiveConfigField::FontSize),
        "default_workspace" => Some(NativeLuaWindowEffectiveConfigField::DefaultWorkspace),
        "default_cwd" => Some(NativeLuaWindowEffectiveConfigField::DefaultCwd),
        "default_domain" => Some(NativeLuaWindowEffectiveConfigField::DefaultDomain),
        "prefer_to_spawn_tabs" => Some(NativeLuaWindowEffectiveConfigField::PreferToSpawnTabs),
        "ssh_backend" => Some(NativeLuaWindowEffectiveConfigField::SshBackend),
        "status_update_interval" => Some(NativeLuaWindowEffectiveConfigField::StatusUpdateInterval),
        "tab_max_width" => Some(NativeLuaWindowEffectiveConfigField::TabMaxWidth),
        "dpi" => Some(NativeLuaWindowEffectiveConfigField::Dpi),
        "color_scheme" => Some(NativeLuaWindowEffectiveConfigField::ColorScheme),
        "foreground_color" => Some(NativeLuaWindowEffectiveConfigField::ForegroundColor),
        "background_color" => Some(NativeLuaWindowEffectiveConfigField::BackgroundColor),
        "max_fps" => Some(NativeLuaWindowEffectiveConfigField::MaxFps),
        "animation_fps" => Some(NativeLuaWindowEffectiveConfigField::AnimationFps),
        "front_end" => Some(NativeLuaWindowEffectiveConfigField::FrontEnd),
        "webgpu_power_preference" => {
            Some(NativeLuaWindowEffectiveConfigField::WebGpuPowerPreference)
        }
        "webgpu_force_fallback_adapter" => {
            Some(NativeLuaWindowEffectiveConfigField::WebGpuForceFallbackAdapter)
        }
        "prefer_egl" => Some(NativeLuaWindowEffectiveConfigField::PreferEgl),
        "enable_wayland" => Some(NativeLuaWindowEffectiveConfigField::EnableWayland),
        "enable_zwlr_output_manager" => {
            Some(NativeLuaWindowEffectiveConfigField::EnableZwlrOutputManager)
        }
        "use_box_model_render" => Some(NativeLuaWindowEffectiveConfigField::UseBoxModelRender),
        "experimental_pixel_positioning" => {
            Some(NativeLuaWindowEffectiveConfigField::ExperimentalPixelPositioning)
        }
        "ignore_svg_fonts" => Some(NativeLuaWindowEffectiveConfigField::IgnoreSvgFonts),
        "bidi_enabled" => Some(NativeLuaWindowEffectiveConfigField::BidiEnabled),
        "bidi_direction" => Some(NativeLuaWindowEffectiveConfigField::BidiDirection),
        "cell_width" => Some(NativeLuaWindowEffectiveConfigField::CellWidth),
        "line_height" => Some(NativeLuaWindowEffectiveConfigField::LineHeight),
        "font_antialias" => Some(NativeLuaWindowEffectiveConfigField::FontAntialias),
        "font_hinting" => Some(NativeLuaWindowEffectiveConfigField::FontHinting),
        "font_rasterizer" => Some(NativeLuaWindowEffectiveConfigField::FontRasterizer),
        "font_colr_rasterizer" => Some(NativeLuaWindowEffectiveConfigField::FontColrRasterizer),
        "font_shaper" => Some(NativeLuaWindowEffectiveConfigField::FontShaper),
        "font_locator" => Some(NativeLuaWindowEffectiveConfigField::FontLocator),
        "use_cap_height_to_scale_fallback_fonts" => {
            Some(NativeLuaWindowEffectiveConfigField::UseCapHeightToScaleFallbackFonts)
        }
        "sort_fallback_fonts_by_coverage" => {
            Some(NativeLuaWindowEffectiveConfigField::SortFallbackFontsByCoverage)
        }
        "search_font_dirs_for_fallback" => {
            Some(NativeLuaWindowEffectiveConfigField::SearchFontDirsForFallback)
        }
        "freetype_load_target" => Some(NativeLuaWindowEffectiveConfigField::FreetypeLoadTarget),
        "freetype_render_target" => Some(NativeLuaWindowEffectiveConfigField::FreetypeRenderTarget),
        "freetype_load_flags" => Some(NativeLuaWindowEffectiveConfigField::FreetypeLoadFlags),
        "freetype_interpreter_version" => {
            Some(NativeLuaWindowEffectiveConfigField::FreetypeInterpreterVersion)
        }
        "freetype_pcf_long_family_names" => {
            Some(NativeLuaWindowEffectiveConfigField::FreetypePcfLongFamilyNames)
        }
        "bold_brightens_ansi_colors" => {
            Some(NativeLuaWindowEffectiveConfigField::BoldBrightensAnsiColors)
        }
        "allow_square_glyphs_to_overflow_width" => {
            Some(NativeLuaWindowEffectiveConfigField::AllowSquareGlyphsToOverflowWidth)
        }
        "display_pixel_geometry" => Some(NativeLuaWindowEffectiveConfigField::DisplayPixelGeometry),
        "text_background_opacity" => {
            Some(NativeLuaWindowEffectiveConfigField::TextBackgroundOpacity)
        }
        "window_background_opacity" => {
            Some(NativeLuaWindowEffectiveConfigField::WindowBackgroundOpacity)
        }
        "shape_cache_size" => Some(NativeLuaWindowEffectiveConfigField::ShapeCacheSize),
        "line_state_cache_size" => Some(NativeLuaWindowEffectiveConfigField::LineStateCacheSize),
        "line_quad_cache_size" => Some(NativeLuaWindowEffectiveConfigField::LineQuadCacheSize),
        "line_to_ele_shape_cache_size" => {
            Some(NativeLuaWindowEffectiveConfigField::LineToEleShapeCacheSize)
        }
        "glyph_cache_image_cache_size" => {
            Some(NativeLuaWindowEffectiveConfigField::GlyphCacheImageCacheSize)
        }
        "cursor_blink_rate" => Some(NativeLuaWindowEffectiveConfigField::CursorBlinkRate),
        "cursor_blink_ease_in" => Some(NativeLuaWindowEffectiveConfigField::CursorBlinkEaseIn),
        "cursor_blink_ease_out" => Some(NativeLuaWindowEffectiveConfigField::CursorBlinkEaseOut),
        "text_blink_rate" => Some(NativeLuaWindowEffectiveConfigField::TextBlinkRate),
        "text_blink_rate_rapid" => Some(NativeLuaWindowEffectiveConfigField::TextBlinkRateRapid),
        "text_blink_ease_in" => Some(NativeLuaWindowEffectiveConfigField::TextBlinkEaseIn),
        "text_blink_ease_out" => Some(NativeLuaWindowEffectiveConfigField::TextBlinkEaseOut),
        "text_blink_rapid_ease_in" => {
            Some(NativeLuaWindowEffectiveConfigField::TextBlinkRapidEaseIn)
        }
        "text_blink_rapid_ease_out" => {
            Some(NativeLuaWindowEffectiveConfigField::TextBlinkRapidEaseOut)
        }
        "cursor_thickness" => Some(NativeLuaWindowEffectiveConfigField::CursorThickness),
        "underline_thickness" => Some(NativeLuaWindowEffectiveConfigField::UnderlineThickness),
        "underline_position" => Some(NativeLuaWindowEffectiveConfigField::UnderlinePosition),
        "strikethrough_position" => {
            Some(NativeLuaWindowEffectiveConfigField::StrikethroughPosition)
        }
        "hide_mouse_cursor_when_typing" => {
            Some(NativeLuaWindowEffectiveConfigField::HideMouseCursorWhenTyping)
        }
        "default_mux_server_domain" => {
            Some(NativeLuaWindowEffectiveConfigField::DefaultMuxServerDomain)
        }
        "ratelimit_mux_line_prefetches_per_second" => {
            Some(NativeLuaWindowEffectiveConfigField::RatelimitMuxLinePrefetchesPerSecond)
        }
        "mux_output_parser_buffer_size" => {
            Some(NativeLuaWindowEffectiveConfigField::MuxOutputParserBufferSize)
        }
        "mux_output_parser_coalesce_delay_ms" => {
            Some(NativeLuaWindowEffectiveConfigField::MuxOutputParserCoalesceDelayMs)
        }
        "periodic_stat_logging" => Some(NativeLuaWindowEffectiveConfigField::PeriodicStatLogging),
        "ulimit_nofile" => Some(NativeLuaWindowEffectiveConfigField::UlimitNofile),
        "ulimit_nproc" => Some(NativeLuaWindowEffectiveConfigField::UlimitNproc),
        "scroll_to_bottom_on_input" => {
            Some(NativeLuaWindowEffectiveConfigField::ScrollToBottomOnInput)
        }
        "use_ime" => Some(NativeLuaWindowEffectiveConfigField::UseIme),
        "xim_im_name" => Some(NativeLuaWindowEffectiveConfigField::XimImName),
        "ime_preedit_rendering" => Some(NativeLuaWindowEffectiveConfigField::ImePreeditRendering),
        "macos_forward_to_ime_modifier_mask" => {
            Some(NativeLuaWindowEffectiveConfigField::MacosForwardToImeModifierMask)
        }
        "notification_handling" => Some(NativeLuaWindowEffectiveConfigField::NotificationHandling),
        "use_dead_keys" => Some(NativeLuaWindowEffectiveConfigField::UseDeadKeys),
        "audible_bell" => Some(NativeLuaWindowEffectiveConfigField::AudibleBell),
        "automatically_reload_config" => {
            Some(NativeLuaWindowEffectiveConfigField::AutomaticallyReloadConfig)
        }
        "check_for_updates" => Some(NativeLuaWindowEffectiveConfigField::CheckForUpdates),
        "show_update_window" => Some(NativeLuaWindowEffectiveConfigField::ShowUpdateWindow),
        "check_for_updates_interval_seconds" => {
            Some(NativeLuaWindowEffectiveConfigField::CheckForUpdatesIntervalSeconds)
        }
        "enable_kitty_graphics" => Some(NativeLuaWindowEffectiveConfigField::EnableKittyGraphics),
        "enable_checksum_rectangular_area" => {
            Some(NativeLuaWindowEffectiveConfigField::EnableChecksumRectangularArea)
        }
        "enable_title_reporting" => Some(NativeLuaWindowEffectiveConfigField::EnableTitleReporting),
        "enable_csi_u_key_encoding" => {
            Some(NativeLuaWindowEffectiveConfigField::EnableCsiUKeyEncoding)
        }
        "enable_kitty_keyboard" => Some(NativeLuaWindowEffectiveConfigField::EnableKittyKeyboard),
        "allow_download_protocols" => {
            Some(NativeLuaWindowEffectiveConfigField::AllowDownloadProtocols)
        }
        "xcursor_theme" => Some(NativeLuaWindowEffectiveConfigField::XcursorTheme),
        "xcursor_size" => Some(NativeLuaWindowEffectiveConfigField::XcursorSize),
        "palette_max_key_assigments_for_action" => {
            Some(NativeLuaWindowEffectiveConfigField::PaletteMaxKeyAssigmentsForAction)
        }
        "allow_win32_input_mode" => Some(NativeLuaWindowEffectiveConfigField::AllowWin32InputMode),
        "treat_left_ctrlalt_as_altgr" => {
            Some(NativeLuaWindowEffectiveConfigField::TreatLeftCtrlAltAsAltGr)
        }
        "send_composed_key_when_left_alt_is_pressed" => {
            Some(NativeLuaWindowEffectiveConfigField::SendComposedKeyWhenLeftAltIsPressed)
        }
        "send_composed_key_when_right_alt_is_pressed" => {
            Some(NativeLuaWindowEffectiveConfigField::SendComposedKeyWhenRightAltIsPressed)
        }
        "treat_east_asian_ambiguous_width_as_wide" => {
            Some(NativeLuaWindowEffectiveConfigField::TreatEastAsianAmbiguousWidthAsWide)
        }
        "normalize_output_to_unicode_nfc" => {
            Some(NativeLuaWindowEffectiveConfigField::NormalizeOutputToUnicodeNfc)
        }
        "unicode_version" => Some(NativeLuaWindowEffectiveConfigField::UnicodeVersion),
        "window_close_confirmation" => {
            Some(NativeLuaWindowEffectiveConfigField::WindowCloseConfirmation)
        }
        _ => None,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the compatibility evaluator remains ordered to preserve Lua precedence"
)]
fn lua_scalar_effective_config_field_part2(
    field: &str,
) -> Option<NativeLuaWindowEffectiveConfigField> {
    match field {
        "enable_tab_bar" => Some(NativeLuaWindowEffectiveConfigField::EnableTabBar),
        "use_fancy_tab_bar" => Some(NativeLuaWindowEffectiveConfigField::UseFancyTabBar),
        "tab_bar_at_bottom" => Some(NativeLuaWindowEffectiveConfigField::TabBarAtBottom),
        "mouse_wheel_scrolls_tabs" => {
            Some(NativeLuaWindowEffectiveConfigField::MouseWheelScrollsTabs)
        }
        "show_close_tab_button_in_tabs" => {
            Some(NativeLuaWindowEffectiveConfigField::ShowCloseTabButtonInTabs)
        }
        "show_new_tab_button_in_tab_bar" => {
            Some(NativeLuaWindowEffectiveConfigField::ShowNewTabButtonInTabBar)
        }
        "show_tab_index_in_tab_bar" => {
            Some(NativeLuaWindowEffectiveConfigField::ShowTabIndexInTabBar)
        }
        "show_tabs_in_tab_bar" => Some(NativeLuaWindowEffectiveConfigField::ShowTabsInTabBar),
        "tab_and_split_indices_are_zero_based" => {
            Some(NativeLuaWindowEffectiveConfigField::TabAndSplitIndicesAreZeroBased)
        }
        "hide_tab_bar_if_only_one_tab" => {
            Some(NativeLuaWindowEffectiveConfigField::HideTabBarIfOnlyOneTab)
        }
        "warn_about_missing_glyphs" => {
            Some(NativeLuaWindowEffectiveConfigField::WarnAboutMissingGlyphs)
        }
        "pane_focus_follows_mouse" => {
            Some(NativeLuaWindowEffectiveConfigField::PaneFocusFollowsMouse)
        }
        "swallow_mouse_click_on_pane_focus" => {
            Some(NativeLuaWindowEffectiveConfigField::SwallowMouseClickOnPaneFocus)
        }
        "swallow_mouse_click_on_window_focus" => {
            Some(NativeLuaWindowEffectiveConfigField::SwallowMouseClickOnWindowFocus)
        }
        "bypass_mouse_reporting_modifiers" => {
            Some(NativeLuaWindowEffectiveConfigField::BypassMouseReportingModifiers)
        }
        "unzoom_on_switch_pane" => Some(NativeLuaWindowEffectiveConfigField::UnzoomOnSwitchPane),
        "quit_when_all_windows_are_closed" => {
            Some(NativeLuaWindowEffectiveConfigField::QuitWhenAllWindowsAreClosed)
        }
        "default_cursor_style" => Some(NativeLuaWindowEffectiveConfigField::DefaultCursorStyle),
        "force_reverse_video_cursor" => {
            Some(NativeLuaWindowEffectiveConfigField::ForceReverseVideoCursor)
        }
        "reverse_video_cursor_min_contrast" => {
            Some(NativeLuaWindowEffectiveConfigField::ReverseVideoCursorMinContrast)
        }
        "text_min_contrast_ratio" => {
            Some(NativeLuaWindowEffectiveConfigField::TextMinContrastRatio)
        }
        "command_palette_rows" => Some(NativeLuaWindowEffectiveConfigField::CommandPaletteRows),
        "command_palette_font_size" => {
            Some(NativeLuaWindowEffectiveConfigField::CommandPaletteFontSize)
        }
        "command_palette_bg_color" => {
            Some(NativeLuaWindowEffectiveConfigField::CommandPaletteBgColor)
        }
        "command_palette_fg_color" => {
            Some(NativeLuaWindowEffectiveConfigField::CommandPaletteFgColor)
        }
        "char_select_font_size" => Some(NativeLuaWindowEffectiveConfigField::CharSelectFontSize),
        "char_select_bg_color" => Some(NativeLuaWindowEffectiveConfigField::CharSelectBgColor),
        "char_select_fg_color" => Some(NativeLuaWindowEffectiveConfigField::CharSelectFgColor),
        "pane_select_font_size" => Some(NativeLuaWindowEffectiveConfigField::PaneSelectFontSize),
        "pane_select_bg_color" => Some(NativeLuaWindowEffectiveConfigField::PaneSelectBgColor),
        "pane_select_fg_color" => Some(NativeLuaWindowEffectiveConfigField::PaneSelectFgColor),
        "launcher_alphabet" => Some(NativeLuaWindowEffectiveConfigField::LauncherAlphabet),
        "quick_select_alphabet" => Some(NativeLuaWindowEffectiveConfigField::QuickSelectAlphabet),
        "disable_default_quick_select_patterns" => {
            Some(NativeLuaWindowEffectiveConfigField::DisableDefaultQuickSelectPatterns)
        }
        "quick_select_remove_styling" => {
            Some(NativeLuaWindowEffectiveConfigField::QuickSelectRemoveStyling)
        }
        "canonicalize_pasted_newlines" => {
            Some(NativeLuaWindowEffectiveConfigField::CanonicalizePastedNewlines)
        }
        "quote_dropped_files" => Some(NativeLuaWindowEffectiveConfigField::QuoteDroppedFiles),
        "disable_default_key_bindings" => {
            Some(NativeLuaWindowEffectiveConfigField::DisableDefaultKeyBindings)
        }
        "disable_default_mouse_bindings" => {
            Some(NativeLuaWindowEffectiveConfigField::DisableDefaultMouseBindings)
        }
        "debug_key_events" => Some(NativeLuaWindowEffectiveConfigField::DebugKeyEvents),
        "key_map_preference" => Some(NativeLuaWindowEffectiveConfigField::KeyMapPreference),
        "ui_key_cap_rendering" => Some(NativeLuaWindowEffectiveConfigField::UiKeyCapRendering),
        "swap_backspace_and_delete" => {
            Some(NativeLuaWindowEffectiveConfigField::SwapBackspaceAndDelete)
        }
        "log_unknown_escape_sequences" => {
            Some(NativeLuaWindowEffectiveConfigField::LogUnknownEscapeSequences)
        }
        "default_ssh_auth_sock" => Some(NativeLuaWindowEffectiveConfigField::DefaultSshAuthSock),
        "mux_enable_ssh_agent" => Some(NativeLuaWindowEffectiveConfigField::MuxEnableSshAgent),
        "detect_password_input" => Some(NativeLuaWindowEffectiveConfigField::DetectPasswordInput),
        "enable_scroll_bar" => Some(NativeLuaWindowEffectiveConfigField::EnableScrollBar),
        "min_scroll_bar_height" => Some(NativeLuaWindowEffectiveConfigField::MinScrollBarHeight),
        "custom_block_glyphs" => Some(NativeLuaWindowEffectiveConfigField::CustomBlockGlyphs),
        "anti_alias_custom_block_glyphs" => {
            Some(NativeLuaWindowEffectiveConfigField::AntiAliasCustomBlockGlyphs)
        }
        "kde_window_background_blur" => {
            Some(NativeLuaWindowEffectiveConfigField::KdeWindowBackgroundBlur)
        }
        "macos_window_background_blur" => {
            Some(NativeLuaWindowEffectiveConfigField::MacosWindowBackgroundBlur)
        }
        "win32_system_backdrop" => Some(NativeLuaWindowEffectiveConfigField::Win32SystemBackdrop),
        "win32_acrylic_accent_color" => {
            Some(NativeLuaWindowEffectiveConfigField::Win32AcrylicAccentColor)
        }
        "window_decorations" => Some(NativeLuaWindowEffectiveConfigField::WindowDecorations),
        "integrated_title_button_alignment" => {
            Some(NativeLuaWindowEffectiveConfigField::IntegratedTitleButtonAlignment)
        }
        "integrated_title_button_color" => {
            Some(NativeLuaWindowEffectiveConfigField::IntegratedTitleButtonColor)
        }
        "integrated_title_button_style" => {
            Some(NativeLuaWindowEffectiveConfigField::IntegratedTitleButtonStyle)
        }
        "native_macos_fullscreen_mode" => {
            Some(NativeLuaWindowEffectiveConfigField::NativeMacosFullscreenMode)
        }
        "macos_fullscreen_extend_behind_notch" => {
            Some(NativeLuaWindowEffectiveConfigField::MacosFullscreenExtendBehindNotch)
        }
        "selection_word_boundary" => {
            Some(NativeLuaWindowEffectiveConfigField::SelectionWordBoundary)
        }
        "term" => Some(NativeLuaWindowEffectiveConfigField::Term),
        "enq_answerback" => Some(NativeLuaWindowEffectiveConfigField::EnqAnswerback),
        "initial_cols" => Some(NativeLuaWindowEffectiveConfigField::InitialCols),
        "initial_rows" => Some(NativeLuaWindowEffectiveConfigField::InitialRows),
        "scrollback_lines" => Some(NativeLuaWindowEffectiveConfigField::ScrollbackLines),
        "switch_to_last_active_tab_when_closing_tab" => {
            Some(NativeLuaWindowEffectiveConfigField::SwitchToLastActiveTabWhenClosingTab)
        }
        "exit_behavior" => Some(NativeLuaWindowEffectiveConfigField::ExitBehavior),
        "exit_behavior_messaging" => {
            Some(NativeLuaWindowEffectiveConfigField::ExitBehaviorMessaging)
        }
        "adjust_window_size_when_changing_font_size" => {
            Some(NativeLuaWindowEffectiveConfigField::AdjustWindowSizeWhenChangingFontSize)
        }
        "use_resize_increments" => Some(NativeLuaWindowEffectiveConfigField::UseResizeIncrements),
        "alternate_buffer_wheel_scroll_speed" => {
            Some(NativeLuaWindowEffectiveConfigField::AlternateBufferWheelScrollSpeed)
        }
        _ => None,
    }
}

fn lua_window_zero_arg_method_name_from_query<'a>(
    value: &'a str,
    window_name: &str,
) -> Option<&'a str> {
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if !lua_config_assignment_field_has_boundaries(rest, 0, method) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty()
        || !lua_trim_start_comments(rest)?.is_empty()
    {
        return None;
    }
    Some(method)
}

fn lua_window_active_tab_zero_arg_method_name_from_query<'a>(
    value: &'a str,
    window_name: &str,
) -> Option<&'a str> {
    lua_window_accessor_zero_arg_method_name_from_query(value, window_name, "active_tab")
}

fn lua_window_active_pane_zero_arg_method_name_from_query<'a>(
    value: &'a str,
    window_name: &str,
) -> Option<&'a str> {
    lua_window_accessor_zero_arg_method_name_from_query(value, window_name, "active_pane")
}

fn lua_window_accessor_zero_arg_method_name_from_query<'a>(
    value: &'a str,
    window_name: &str,
    accessor_name: &str,
) -> Option<&'a str> {
    let rest = value.strip_prefix(window_name)?;
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return None;
    }
    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let accessor = lua_identifier_literal_from_query(rest)?;
    if accessor != accessor_name || !lua_config_assignment_field_has_boundaries(rest, 0, accessor) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(accessor.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty() {
        return None;
    }

    let rest = lua_trim_start_comments(rest)?.strip_prefix(':')?;
    let rest = lua_trim_start_comments(rest)?;
    let method = lua_identifier_literal_from_query(rest)?;
    if !lua_config_assignment_field_has_boundaries(rest, 0, method) {
        return None;
    }
    let rest = lua_trim_start_comments(rest.get(method.len()..)?)?;
    let rest = rest.strip_prefix('(')?;
    let (arguments, rest) = lua_parenthesized_argument_list_prefix_from_query(rest)?;
    if !lua_trim_start_comments(arguments)?.trim().is_empty()
        || !lua_trim_start_comments(rest)?.is_empty()
    {
        return None;
    }
    Some(method)
}
