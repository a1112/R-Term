#[allow(dead_code)]
fn lua_config_bracket_assignment_rest_from_query<'a>(
    source: &'a str,
    start: usize,
    receiver: &str,
    field: &str,
) -> Option<&'a str> {
    if !lua_config_bracket_assignment_has_receiver(source, start, receiver) {
        return None;
    }

    let target = source.get(start..)?;
    let after_open = lua_trim_start_comments(target.strip_prefix('[')?)?;
    let (key, after_key) = lua_config_bracket_assignment_key_from_query(source, after_open, start)?;
    if key != field {
        return None;
    }

    lua_trim_start_comments(after_key)?.strip_prefix(']')
}
fn lua_config_bracket_assignment_key_from_query<'a>(
    source: &'a str,
    query: &'a str,
    max_start: usize,
) -> Option<(String, &'a str)> {
    if let Some(key_literal) = lua_quoted_string_literal_from_query(query)
        .or_else(|| lua_long_bracket_literal_from_query(query))
    {
        return Some((
            parse_maybe_quoted_query_text(key_literal)?,
            query.get(key_literal.len()..)?,
        ));
    }

    let variable = lua_identifier_literal_from_query(query)?;
    let key_literal = lua_static_string_variable_assignment_before_offset_from_query(
        source, variable, max_start,
    )?;
    Some((
        parse_maybe_quoted_query_text(key_literal)?,
        query.get(variable.len()..)?,
    ))
}

#[allow(dead_code)]
fn lua_config_bracket_assignment_has_config_receiver(source: &str, start: usize) -> bool {
    lua_config_bracket_assignment_has_receiver(source, start, "config")
}

fn lua_config_bracket_assignment_has_receiver(source: &str, start: usize, receiver: &str) -> bool {
    let prefix = source[..start].trim_end();
    let Some(receiver_start) = prefix.len().checked_sub(receiver.len()) else {
        return false;
    };
    if &prefix[receiver_start..] != receiver {
        return false;
    }
    let before_receiver = prefix[..receiver_start].chars().next_back();
    !before_receiver.is_some_and(is_lua_identifier_character)
}

#[allow(dead_code)]
fn lua_bool_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    for value in ["false", "true"] {
        if let Some(rest) = query.strip_prefix(value) {
            let next = rest.chars().next();
            if !next.is_some_and(is_lua_identifier_character) {
                return query.get(..value.len());
            }
        }
    }
    None
}

#[allow(dead_code)]
fn lua_unsigned_integer_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    let end = query
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_digit())
        .map(|(index, character)| index + character.len_utf8())
        .last()?;
    let rest = &query[end..];
    let next = rest.chars().next();
    (!next.is_some_and(is_lua_identifier_character) && !query[..end].is_empty())
        .then_some(&query[..end])
}

#[allow(dead_code)]
fn lua_unsigned_number_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    let mut end = 0;
    let mut digits = 0;
    let mut decimal_seen = false;

    for (index, character) in query.char_indices() {
        if character.is_ascii_digit() {
            digits += 1;
            end = index + character.len_utf8();
        } else if character == '.' && !decimal_seen {
            decimal_seen = true;
            end = index + character.len_utf8();
        } else {
            break;
        }
    }

    if digits == 0 {
        return None;
    }

    let rest = &query[end..];
    let next = rest.chars().next();
    (!next.is_some_and(is_lua_identifier_character) && next != Some('.')).then_some(&query[..end])
}

#[allow(dead_code)]
fn lua_signed_number_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    let sign_len = query
        .chars()
        .next()
        .filter(|character| *character == '-')
        .map_or(0, char::len_utf8);
    let number = query.get(sign_len..)?;
    let mut end = 0;
    let mut digits = 0;
    let mut decimal_seen = false;

    for (index, character) in number.char_indices() {
        if character.is_ascii_digit() {
            digits += 1;
            end = sign_len + index + character.len_utf8();
        } else if character == '.' && !decimal_seen {
            decimal_seen = true;
            end = sign_len + index + character.len_utf8();
        } else {
            break;
        }
    }

    if digits == 0 {
        return None;
    }

    let rest = &query[end..];
    let next = rest.chars().next();
    (!next.is_some_and(is_lua_identifier_character) && next != Some('.')).then_some(&query[..end])
}

#[allow(dead_code)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "finite positive point sizes are rounded and bounded before conversion"
)]
fn native_font_size_from_points(points: f32) -> Option<NativeFontSize> {
    if !points.is_finite() || points <= 0.0 {
        return None;
    }
    let millipoints = (points * 1_000.0).round();
    (millipoints <= 4_294_967_296.0).then(|| NativeFontSize::from_millipoints(millipoints as u32))
}

fn native_lua_font_size_points_text(font_size: NativeFontSize) -> String {
    let points = font_size.millipoints / 1_000;
    let millipoints = font_size.millipoints % 1_000;
    if millipoints == 0 {
        return points.to_string();
    }

    let mut fraction = format!("{millipoints:03}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{points}.{fraction}")
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "finite positive DPI values are rounded and bounded before conversion"
)]
fn native_dpi_from_f32(dpi: f32) -> Option<u32> {
    if !dpi.is_finite() || dpi <= 0.0 {
        return None;
    }
    let dpi = dpi.round();
    (dpi <= 4_294_967_296.0).then_some(dpi as u32)
}

#[allow(dead_code)]
fn native_dpi_by_screen_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<BTreeMap<String, u32>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut dpi_by_screen = BTreeMap::new();
    let static_source = Some(LuaStaticSource { source, max_start });

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = lua_static_number_assignment_value_before_offset_from_query(
            source,
            value.trim(),
            max_start,
            lua_unsigned_number_literal_from_query,
        )?;
        let dpi = native_dpi_from_f32(value.parse().ok()?)?;
        dpi_by_screen.insert(key, dpi);
    }

    Some(dpi_by_screen)
}

#[allow(dead_code)]
fn native_exec_domains_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeExecDomain>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut domains = Vec::new();
    let mut indexed_domains = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !domains.is_empty() || index == 0 || indexed_domains.contains_key(&index) {
                return None;
            }
            indexed_domains.insert(
                index,
                native_exec_domain_lua_value_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_domains.is_empty() {
            return None;
        }
        domains.push(native_exec_domain_lua_value_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_domains.is_empty() {
        return (1..=indexed_domains.len())
            .map(|index| indexed_domains.remove(&index))
            .collect();
    }

    Some(domains)
}

#[allow(dead_code)]
fn native_exec_domain_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeExecDomain> {
    let value = value.trim();
    if let Some(domain) = native_exec_domain_function_from_query(static_source, value) {
        return Some(domain);
    }
    if value.starts_with('{') {
        return native_exec_domain_lua_table_from_query(static_source, value);
    }

    let static_source = static_source?;
    let expression = lua_static_expression_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
    )?;
    native_exec_domain_lua_value_from_query(Some(static_source), expression)
}

#[allow(dead_code)]
fn native_exec_domain_function_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeExecDomain> {
    let body = strip_lua_function_call_from_query(value, "wezterm.exec_domain")
        .or_else(|| strip_lua_function_call_from_query(value, "exec_domain"))?;
    let args = split_lua_top_level_arguments(body)?;
    if !(2..=3).contains(&args.len()) {
        return None;
    }

    let name = parse_maybe_static_query_text(static_source, args[0].trim())?;
    let name = non_empty_spawn_command_option_value(&name).ok()?;
    if !native_lua_function_expression_from_query(args[1]) {
        return None;
    }
    let label = if let Some(label) = args.get(2) {
        Some(native_exec_domain_label_from_query(
            static_source,
            &name,
            label.trim(),
        )?)
    } else {
        None
    };

    Some(NativeExecDomain {
        fixup_command: format!("exec-domain-{name}"),
        name,
        label,
    })
}

#[allow(dead_code)]
fn native_exec_domain_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeExecDomain> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut name = None;
    let mut fixup_command = None;
    let mut label = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        if key.eq_ignore_ascii_case("name") {
            if name.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            name = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("fixup_command") {
            if fixup_command.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            fixup_command = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("label") {
            if label.is_some() {
                return None;
            }
            let current_name = name.as_deref().unwrap_or_default();
            label = Some(native_exec_domain_label_from_query(
                static_source,
                current_name,
                value,
            )?);
        } else {
            return None;
        }
    }

    Some(NativeExecDomain {
        name: name?,
        fixup_command: fixup_command?,
        label,
    })
}

#[allow(dead_code)]
fn native_exec_domain_label_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    name: &str,
    value: &str,
) -> Option<NativeExecDomainLabel> {
    if native_lua_function_expression_from_query(value) {
        return Some(NativeExecDomainLabel::Function(format!(
            "exec-domain-{name}-label"
        )));
    }

    let value = parse_maybe_static_query_text(static_source, value)?;
    Some(NativeExecDomainLabel::Value(
        non_empty_spawn_command_option_value(&value).ok()?,
    ))
}

#[allow(dead_code)]
fn native_lua_function_expression_from_query(value: &str) -> bool {
    let value = value.trim_start();
    lua_source_keyword_at(value, 0, "function")
}

#[allow(dead_code)]
fn native_wsl_domains_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeWslDomain>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut domains = Vec::new();
    let mut indexed_domains = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !domains.is_empty() || index == 0 || indexed_domains.contains_key(&index) {
                return None;
            }
            indexed_domains.insert(
                index,
                native_wsl_domain_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_domains.is_empty() {
            return None;
        }
        domains.push(native_wsl_domain_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_domains.is_empty() {
        return (1..=indexed_domains.len())
            .map(|index| indexed_domains.remove(&index))
            .collect();
    }

    Some(domains)
}

#[allow(dead_code)]
fn native_wsl_domain_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeWslDomain> {
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
    let mut name = None;
    let mut distribution = None;
    let mut username = None;
    let mut default_cwd = None;
    let mut default_prog = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        if key.eq_ignore_ascii_case("name") {
            if name.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            name = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("distribution") {
            if distribution.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            distribution = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("username") {
            if username.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            username = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("default_cwd") {
            if default_cwd.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            default_cwd = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("default_prog") {
            if default_prog.is_some() {
                return None;
            }
            default_prog = Some(split_lua_table_string_array_with_static_source(
                static_source,
                value,
            )?);
        } else {
            return None;
        }
    }

    Some(NativeWslDomain {
        name: name?,
        distribution,
        username,
        default_cwd,
        default_prog,
    })
}

#[allow(dead_code)]
fn native_unix_domains_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeUnixDomain>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut domains = Vec::new();
    let mut indexed_domains = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !domains.is_empty() || index == 0 || indexed_domains.contains_key(&index) {
                return None;
            }
            indexed_domains.insert(
                index,
                native_unix_domain_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_domains.is_empty() {
            return None;
        }
        domains.push(native_unix_domain_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_domains.is_empty() {
        return (1..=indexed_domains.len())
            .map(|index| indexed_domains.remove(&index))
            .collect();
    }

    Some(domains)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
#[allow(dead_code)]
fn native_unix_domain_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeUnixDomain> {
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
    let mut name = None;
    let mut socket_path = None;
    let mut connect_automatically = None;
    let mut no_serve_automatically = None;
    let mut serve_command = None;
    let mut proxy_command = None;
    let mut skip_permissions_check = None;
    let mut read_timeout_ms = None;
    let mut write_timeout_ms = None;
    let mut local_echo_threshold_ms = None;
    let mut overlay_lag_indicator = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "name" => {
                if name.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                name = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "socket_path" => {
                if socket_path.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                socket_path = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "connect_automatically" => {
                if connect_automatically.is_some() {
                    return None;
                }
                connect_automatically = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "no_serve_automatically" => {
                if no_serve_automatically.is_some() {
                    return None;
                }
                no_serve_automatically = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "serve_command" => {
                if serve_command.is_some() {
                    return None;
                }
                serve_command = Some(split_lua_table_string_array_with_static_source(
                    static_source,
                    value,
                )?);
            }
            "proxy_command" => {
                if proxy_command.is_some() {
                    return None;
                }
                proxy_command = Some(split_lua_table_string_array_with_static_source(
                    static_source,
                    value,
                )?);
            }
            "skip_permissions_check" => {
                if skip_permissions_check.is_some() {
                    return None;
                }
                skip_permissions_check = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "read_timeout" | "read_timeout_ms" => {
                if read_timeout_ms.is_some() {
                    return None;
                }
                read_timeout_ms = Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "write_timeout" | "write_timeout_ms" => {
                if write_timeout_ms.is_some() {
                    return None;
                }
                write_timeout_ms = Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "local_echo_threshold_ms" => {
                if local_echo_threshold_ms.is_some() {
                    return None;
                }
                local_echo_threshold_ms =
                    Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "overlay_lag_indicator" => {
                if overlay_lag_indicator.is_some() {
                    return None;
                }
                overlay_lag_indicator = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            _ => return None,
        }
    }

    Some(NativeUnixDomain {
        name: name?,
        socket_path,
        connect_automatically: connect_automatically.unwrap_or(false),
        no_serve_automatically: no_serve_automatically.unwrap_or(false),
        serve_command,
        proxy_command,
        skip_permissions_check: skip_permissions_check.unwrap_or(false),
        read_timeout_ms: read_timeout_ms.unwrap_or(DEFAULT_UNIX_DOMAIN_TIMEOUT_MS),
        write_timeout_ms: write_timeout_ms.unwrap_or(DEFAULT_UNIX_DOMAIN_TIMEOUT_MS),
        local_echo_threshold_ms,
        overlay_lag_indicator: overlay_lag_indicator.unwrap_or(false),
    })
}

#[allow(dead_code)]
fn native_lua_static_u64_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<u64> {
    let static_source = static_source?;
    lua_static_number_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
        lua_unsigned_integer_literal_from_query,
    )?
    .parse()
    .ok()
}

#[allow(dead_code)]
fn native_ssh_domains_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeSshDomain>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut domains = Vec::new();
    let mut indexed_domains = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !domains.is_empty() || index == 0 || indexed_domains.contains_key(&index) {
                return None;
            }
            indexed_domains.insert(
                index,
                native_ssh_domain_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_domains.is_empty() {
            return None;
        }
        domains.push(native_ssh_domain_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_domains.is_empty() {
        return (1..=indexed_domains.len())
            .map(|index| indexed_domains.remove(&index))
            .collect();
    }

    Some(domains)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
#[allow(dead_code)]
fn native_ssh_domain_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeSshDomain> {
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
    let mut name = None;
    let mut remote_address = None;
    let mut no_agent_auth = None;
    let mut username = None;
    let mut connect_automatically = None;
    let mut timeout_ms = None;
    let mut local_echo_threshold_ms = None;
    let mut overlay_lag_indicator = None;
    let mut remote_wezterm_path = None;
    let mut override_proxy_command = None;
    let mut ssh_backend = None;
    let mut multiplexing = None;
    let mut ssh_option = None;
    let mut default_prog = None;
    let mut assume_shell = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "name" => {
                if name.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                name = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "remote_address" => {
                if remote_address.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                remote_address = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "no_agent_auth" => {
                if no_agent_auth.is_some() {
                    return None;
                }
                no_agent_auth = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "username" => {
                if username.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                username = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "connect_automatically" => {
                if connect_automatically.is_some() {
                    return None;
                }
                connect_automatically = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "timeout" | "timeout_ms" => {
                if timeout_ms.is_some() {
                    return None;
                }
                timeout_ms = Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "local_echo_threshold_ms" => {
                if local_echo_threshold_ms.is_some() {
                    return None;
                }
                local_echo_threshold_ms =
                    Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "overlay_lag_indicator" => {
                if overlay_lag_indicator.is_some() {
                    return None;
                }
                overlay_lag_indicator = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "remote_wezterm_path" => {
                if remote_wezterm_path.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                remote_wezterm_path = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "override_proxy_command" => {
                if override_proxy_command.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                override_proxy_command = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "ssh_backend" => {
                if ssh_backend.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                ssh_backend = Some(NativeSshBackend::parse(&value)?);
            }
            "multiplexing" => {
                if multiplexing.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                multiplexing = Some(NativeSshMultiplexing::parse(&value)?);
            }
            "ssh_option" => {
                if ssh_option.is_some() {
                    return None;
                }
                ssh_option = Some(native_lua_static_string_map_from_query(
                    static_source,
                    value,
                )?);
            }
            "default_prog" => {
                if default_prog.is_some() {
                    return None;
                }
                default_prog = Some(split_lua_table_string_array_with_static_source(
                    static_source,
                    value,
                )?);
            }
            "assume_shell" => {
                if assume_shell.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                assume_shell = Some(NativeShellAssumption::parse(&value)?);
            }
            _ => return None,
        }
    }

    Some(NativeSshDomain {
        name: name?,
        remote_address: remote_address?,
        no_agent_auth: no_agent_auth.unwrap_or(false),
        username,
        connect_automatically: connect_automatically.unwrap_or(false),
        timeout_ms: timeout_ms.unwrap_or(DEFAULT_SSH_DOMAIN_TIMEOUT_MS),
        local_echo_threshold_ms: local_echo_threshold_ms
            .or(Some(DEFAULT_SSH_DOMAIN_LOCAL_ECHO_THRESHOLD_MS)),
        overlay_lag_indicator: overlay_lag_indicator.unwrap_or(false),
        remote_wezterm_path,
        override_proxy_command,
        ssh_backend,
        multiplexing: multiplexing.unwrap_or_default(),
        ssh_option: ssh_option.unwrap_or_default(),
        default_prog,
        assume_shell: assume_shell.unwrap_or_default(),
    })
}

#[allow(dead_code)]
fn native_lua_static_string_map_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<BTreeMap<String, String>> {
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
    split_lua_table_environment_from_query_with_static_source(static_source, value)
}

#[allow(dead_code)]
fn native_tls_server_domains_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeTlsServerDomain>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut domains = Vec::new();
    let mut indexed_domains = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !domains.is_empty() || index == 0 || indexed_domains.contains_key(&index) {
                return None;
            }
            indexed_domains.insert(
                index,
                native_tls_server_domain_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_domains.is_empty() {
            return None;
        }
        domains.push(native_tls_server_domain_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_domains.is_empty() {
        return (1..=indexed_domains.len())
            .map(|index| indexed_domains.remove(&index))
            .collect();
    }

    Some(domains)
}

#[allow(dead_code)]
fn native_tls_server_domain_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeTlsServerDomain> {
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
    let mut domain = NativeTlsServerDomain::default();
    let mut bind_address = None;
    let mut pem_private_key = None;
    let mut pem_cert = None;
    let mut pem_ca = None;
    let mut pem_root_certs = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "bind_address" => {
                if bind_address.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                domain.bind_address = non_empty_spawn_command_option_value(&value).ok()?;
                bind_address = Some(());
            }
            "pem_private_key" => {
                if pem_private_key.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                domain.pem_private_key = Some(non_empty_spawn_command_option_value(&value).ok()?);
                pem_private_key = Some(());
            }
            "pem_cert" => {
                if pem_cert.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                domain.pem_cert = Some(non_empty_spawn_command_option_value(&value).ok()?);
                pem_cert = Some(());
            }
            "pem_ca" => {
                if pem_ca.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                domain.pem_ca = Some(non_empty_spawn_command_option_value(&value).ok()?);
                pem_ca = Some(());
            }
            "pem_root_certs" => {
                if pem_root_certs.is_some() {
                    return None;
                }
                domain.pem_root_certs =
                    split_lua_table_string_array_with_static_source(static_source, value)?;
                pem_root_certs = Some(());
            }
            _ => return None,
        }
    }

    bind_address?;
    Some(domain)
}

#[allow(dead_code)]
fn native_tls_client_domains_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeTlsClientDomain>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut domains = Vec::new();
    let mut indexed_domains = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !domains.is_empty() || index == 0 || indexed_domains.contains_key(&index) {
                return None;
            }
            indexed_domains.insert(
                index,
                native_tls_client_domain_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_domains.is_empty() {
            return None;
        }
        domains.push(native_tls_client_domain_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_domains.is_empty() {
        return (1..=indexed_domains.len())
            .map(|index| indexed_domains.remove(&index))
            .collect();
    }

    Some(domains)
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
#[allow(dead_code)]
fn native_tls_client_domain_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeTlsClientDomain> {
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
    let mut name = None;
    let mut bootstrap_via_ssh = None;
    let mut remote_address = None;
    let mut pem_private_key = None;
    let mut pem_cert = None;
    let mut pem_ca = None;
    let mut pem_root_certs = None;
    let mut accept_invalid_hostnames = None;
    let mut expected_cn = None;
    let mut connect_automatically = None;
    let mut read_timeout_ms = None;
    let mut write_timeout_ms = None;
    let mut local_echo_threshold_ms = None;
    let mut remote_wezterm_path = None;
    let mut overlay_lag_indicator = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "name" => {
                if name.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                name = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "bootstrap_via_ssh" => {
                if bootstrap_via_ssh.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                bootstrap_via_ssh = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "remote_address" => {
                if remote_address.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                remote_address = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "pem_private_key" => {
                if pem_private_key.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                pem_private_key = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "pem_cert" => {
                if pem_cert.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                pem_cert = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "pem_ca" => {
                if pem_ca.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                pem_ca = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "pem_root_certs" => {
                if pem_root_certs.is_some() {
                    return None;
                }
                pem_root_certs = Some(split_lua_table_string_array_with_static_source(
                    static_source,
                    value,
                )?);
            }
            "accept_invalid_hostnames" => {
                if accept_invalid_hostnames.is_some() {
                    return None;
                }
                accept_invalid_hostnames =
                    Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "expected_cn" => {
                if expected_cn.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                expected_cn = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "connect_automatically" => {
                if connect_automatically.is_some() {
                    return None;
                }
                connect_automatically = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            "read_timeout" | "read_timeout_ms" => {
                if read_timeout_ms.is_some() {
                    return None;
                }
                read_timeout_ms = Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "write_timeout" | "write_timeout_ms" => {
                if write_timeout_ms.is_some() {
                    return None;
                }
                write_timeout_ms = Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "local_echo_threshold_ms" => {
                if local_echo_threshold_ms.is_some() {
                    return None;
                }
                local_echo_threshold_ms =
                    Some(native_lua_static_u64_from_query(static_source, value)?);
            }
            "remote_wezterm_path" => {
                if remote_wezterm_path.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                remote_wezterm_path = Some(non_empty_spawn_command_option_value(&value).ok()?);
            }
            "overlay_lag_indicator" => {
                if overlay_lag_indicator.is_some() {
                    return None;
                }
                overlay_lag_indicator = Some(parse_maybe_static_query_bool(static_source, value)?);
            }
            _ => return None,
        }
    }

    Some(NativeTlsClientDomain {
        name: name?,
        bootstrap_via_ssh,
        remote_address: remote_address?,
        pem_private_key,
        pem_cert,
        pem_ca,
        pem_root_certs: pem_root_certs.unwrap_or_default(),
        accept_invalid_hostnames: accept_invalid_hostnames.unwrap_or(false),
        expected_cn,
        connect_automatically: connect_automatically.unwrap_or(false),
        read_timeout_ms: read_timeout_ms.unwrap_or(DEFAULT_TLS_DOMAIN_TIMEOUT_MS),
        write_timeout_ms: write_timeout_ms.unwrap_or(DEFAULT_TLS_DOMAIN_TIMEOUT_MS),
        local_echo_threshold_ms: local_echo_threshold_ms
            .or(Some(DEFAULT_TLS_DOMAIN_LOCAL_ECHO_THRESHOLD_MS)),
        remote_wezterm_path,
        overlay_lag_indicator: overlay_lag_indicator.unwrap_or(false),
    })
}

#[allow(dead_code)]
fn native_serial_ports_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeSerialDomain>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut ports = Vec::new();
    let mut indexed_ports = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !ports.is_empty() || index == 0 || indexed_ports.contains_key(&index) {
                return None;
            }
            indexed_ports.insert(
                index,
                native_serial_domain_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_ports.is_empty() {
            return None;
        }
        ports.push(native_serial_domain_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_ports.is_empty() {
        return (1..=indexed_ports.len())
            .map(|index| indexed_ports.remove(&index))
            .collect();
    }

    Some(ports)
}

#[allow(dead_code)]
fn native_serial_domain_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeSerialDomain> {
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
    let mut name = None;
    let mut port = None;
    let mut baud = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        if key.eq_ignore_ascii_case("name") {
            if name.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            name = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("port") {
            if port.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value)?;
            port = Some(non_empty_spawn_command_option_value(&value).ok()?);
        } else if key.eq_ignore_ascii_case("baud") {
            if baud.is_some() {
                return None;
            }
            baud = Some(
                lua_static_number_assignment_value_before_offset_from_query(
                    static_source?.source,
                    value,
                    static_source?.max_start,
                    lua_unsigned_integer_literal_from_query,
                )?
                .parse()
                .ok()?,
            );
        } else {
            return None;
        }
    }

    Some(NativeSerialDomain {
        name: name?,
        port,
        baud,
    })
}

#[allow(dead_code)]
fn native_cell_width_from_ratio(ratio: f32) -> Option<NativeCellWidth> {
    native_ratio_to_per_mille(ratio).map(NativeCellWidth::from_per_mille)
}

#[allow(dead_code)]
fn native_cell_widths_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: usize,
) -> Option<Vec<NativeCellWidthOverride>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut overrides = Vec::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let entry =
            split_lua_table_assignment_from_field(field).map_or(field, |(_, value)| value.trim());
        overrides.push(native_cell_width_override_lua_table_from_query(
            source, entry, max_start,
        )?);
    }

    Some(overrides)
}

#[allow(dead_code)]
fn native_cell_width_override_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: usize,
) -> Option<NativeCellWidthOverride> {
    let value = value.trim();
    let resolved_value;
    let value = if value.starts_with('{') {
        value
    } else {
        resolved_value = lua_table_insert_value_table_string_from_query(source, value, max_start)?;
        resolved_value.as_str()
    };
    let table = value.strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut first = None;
    let mut last = None;
    let mut width = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "first" => {
                first = Some(lua_static_unsigned_u32_value_before_offset_from_query(
                    source, value, max_start,
                )?);
            }
            "last" => {
                last = Some(lua_static_unsigned_u32_value_before_offset_from_query(
                    source, value, max_start,
                )?);
            }
            "width" => {
                width = Some(
                    u16::try_from(lua_static_unsigned_u32_value_before_offset_from_query(
                        source, value, max_start,
                    )?)
                    .ok()?,
                );
            }
            _ => return None,
        }
    }

    let first = first?;
    let last = last?;
    let width = width?;
    (first <= last).then(|| NativeCellWidthOverride::new(first, last, width))
}

#[allow(dead_code)]
fn lua_static_unsigned_u32_value_from_query<'a>(source: &'a str, value: &'a str) -> Option<u32> {
    let value = lua_static_number_assignment_value_from_query(
        source,
        value,
        lua_unsigned_u32_literal_from_query,
    )?;
    lua_unsigned_u32_value_from_query(value)
}

fn lua_static_unsigned_u32_value_before_offset_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: usize,
) -> Option<u32> {
    if let Some(value) = lua_unsigned_u32_literal_from_query(value) {
        return lua_unsigned_u32_value_from_query(value);
    }

    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let value = lua_static_number_variable_assignment_before_offset_from_query(
        source,
        variable,
        max_start,
        lua_unsigned_u32_literal_from_query,
    )?;
    lua_unsigned_u32_value_from_query(value)
}

#[allow(dead_code)]
fn lua_unsigned_u32_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    if let Some(rest) = query
        .strip_prefix("0x")
        .or_else(|| query.strip_prefix("0X"))
    {
        let end = rest
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_hexdigit())
            .map(|(index, character)| index + character.len_utf8())
            .last()?;
        let trailing = rest[end..].chars().next();
        if trailing.is_some_and(is_lua_identifier_character) || end == 0 {
            return None;
        }
        return query.get(..2 + end);
    }

    lua_unsigned_integer_literal_from_query(query)
}

#[allow(dead_code)]
fn lua_unsigned_u32_value_from_query(value: &str) -> Option<u32> {
    let value = value.trim();
    if let Some(rest) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        let end = rest
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_hexdigit())
            .map(|(index, character)| index + character.len_utf8())
            .last()?;
        let trailing = rest[end..].chars().next();
        if trailing.is_some_and(is_lua_identifier_character) || end == 0 {
            return None;
        }
        return u32::from_str_radix(&rest[..end], 16).ok();
    }

    lua_unsigned_integer_literal_from_query(value)?.parse().ok()
}

#[allow(dead_code)]
fn native_hyperlink_rules_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeHyperlinkRule>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut rules = Vec::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let entry =
            split_lua_table_assignment_from_field(field).map_or(field, |(_, value)| value.trim());
        rules.push(native_hyperlink_rule_lua_table_from_query(
            source, entry, max_start,
        )?);
    }

    Some(rules)
}

fn lua_config_hyperlink_rules_extends_default_rules_before_offset(
    source: &str,
    max_start: usize,
) -> Option<bool> {
    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let starts = lua_top_level_statement_start_indices_before_offset(source, max_start)?;
    let mut extends_default = false;

    if let Some(table) = lua_config_static_return_table_from_query(source)
        && lua_source_slice_start_offset(source, table) == Some(max_start)
    {
        extends_default |=
            lua_config_table_hyperlink_rules_extends_default_rules(source, table, max_start)
                .unwrap_or(false);
    }

    if let Some(returned_table) =
        lua_static_table_variable_assignment_before_offset_from_query(source, receiver, max_start)
            .or_else(|| {
                lua_static_table_variable_assignment_at_offset_from_query(
                    source, receiver, max_start,
                )
            })
    {
        extends_default |= lua_config_table_hyperlink_rules_extends_default_rules(
            source,
            returned_table
                .trim()
                .strip_prefix('{')?
                .strip_suffix('}')?
                .trim(),
            max_start,
        )
        .unwrap_or(false);
    }

    for (position, start) in starts.iter().copied().enumerate() {
        let statement_end = starts.get(position + 1).copied().unwrap_or(max_start);
        let statement = source.get(start..statement_end)?;
        let after_receiver = if lua_source_keyword_at(source, start, "local") {
            let rest = lua_trim_start_comments(source.get(start + "local".len()..)?)?;
            lua_config_receiver_prefix_rest(rest, receiver)
        } else {
            lua_config_receiver_prefix_rest(statement, receiver)
        };
        let Some(after_receiver) = after_receiver else {
            continue;
        };
        let Some(after_field) = lua_config_field_access_rest_from_query_with_static_key(
            source,
            after_receiver,
            "hyperlink_rules",
            start,
        ) else {
            continue;
        };
        let after_field = lua_trim_start_comments(after_field)?;
        let Some(value) = after_field.strip_prefix('=') else {
            continue;
        };
        extends_default = lua_wezterm_default_hyperlink_rules_value_from_query_with_static_source(
            source,
            lua_trim_start_comments(value)?,
            start,
        );
    }

    Some(extends_default)
}

fn lua_config_hyperlink_rules_default_rules_with_static_inserts(
    source: &str,
    max_start: usize,
) -> Option<Vec<NativeHyperlinkRule>> {
    let variable =
        lua_config_hyperlink_rules_static_value_variable_before_offset(source, max_start)?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        source, variable, max_start,
    )?;
    if !lua_wezterm_default_hyperlink_rules_value_from_query(value) {
        return None;
    }

    let mut rules = default_hyperlink_rules();
    let mut inserted = false;
    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if let Some(insert) =
            lua_static_table_variable_insert_append_value_from_query(source, start, variable)
        {
            apply_hyperlink_rule_insert(&mut rules, source, &insert, start)?;
            inserted = true;
        } else if let Some(assignment) =
            lua_static_table_variable_index_assignment_from_query(source, start, variable)
        {
            let assignment = LuaTableIndexOrAppendAssignment {
                index: Some(assignment.index),
                value: assignment.value,
            };
            apply_hyperlink_rule_index_or_append_assignment(
                &mut rules,
                source,
                &assignment,
                start,
            )?;
            inserted = true;
        } else if let Some(assignment) =
            lua_static_table_variable_indexed_field_assignment_from_query(source, start, variable)
            && apply_hyperlink_rule_indexed_field_assignment(
                &mut rules,
                source,
                &assignment,
                start,
            )?
        {
            inserted = true;
        }
    }

    inserted.then_some(rules)
}

fn lua_config_hyperlink_rules_static_default_alias_before_offset(
    source: &str,
    max_start: usize,
) -> Option<bool> {
    let variable =
        lua_config_hyperlink_rules_static_value_variable_before_offset(source, max_start)?;
    let value = lua_static_expression_variable_assignment_before_offset_from_query(
        source, variable, max_start,
    )?;
    Some(lua_wezterm_default_hyperlink_rules_value_from_query(value))
}

fn lua_config_hyperlink_rules_returned_table_default_value_before_offset(
    source: &str,
    max_start: usize,
) -> Option<bool> {
    if let Some(table) = lua_config_static_return_table_from_query(source)
        && lua_source_slice_start_offset(source, table) == Some(max_start)
        && lua_config_table_hyperlink_rules_extends_default_rules(source, table, max_start)
            .unwrap_or(false)
    {
        return Some(true);
    }

    let receiver = lua_config_static_return_identifier_from_query(source)?;
    let Some(returned_table) =
        lua_static_table_variable_assignment_before_offset_from_query(source, receiver, max_start)
            .or_else(|| {
                lua_static_table_variable_assignment_at_offset_from_query(
                    source, receiver, max_start,
                )
            })
    else {
        return Some(false);
    };
    lua_config_table_hyperlink_rules_extends_default_rules(
        source,
        returned_table
            .trim()
            .strip_prefix('{')?
            .strip_suffix('}')?
            .trim(),
        max_start,
    )
}

fn lua_config_hyperlink_rules_default_rules_with_config_inserts(
    source: &str,
    max_start: usize,
) -> Option<Vec<NativeHyperlinkRule>> {
    if !lua_config_hyperlink_rules_extends_default_rules_before_offset(source, max_start)? {
        return None;
    }

    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let mut rules = default_hyperlink_rules();
    let mut inserted = false;
    for start in lua_top_level_statement_start_indices_before_offset(source, max_start)? {
        if apply_config_hyperlink_rule_assignment_at_start(&mut rules, source, start, receiver)? {
            inserted = true;
        }
    }
    if apply_config_hyperlink_rule_assignment_at_start(&mut rules, source, max_start, receiver)? {
        inserted = true;
    }

    inserted.then_some(rules)
}

fn lua_config_hyperlink_rules_direct_default_assignment_before_offset(
    source: &str,
    max_start: usize,
) -> Option<bool> {
    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    let starts = lua_top_level_statement_start_indices_before_offset(source, max_start)?;

    for (position, start) in starts.iter().copied().enumerate() {
        let statement_end = starts.get(position + 1).copied().unwrap_or(max_start);
        let statement = source.get(start..statement_end)?;
        let after_receiver = if lua_source_keyword_at(source, start, "local") {
            let rest = lua_trim_start_comments(source.get(start + "local".len()..)?)?;
            lua_config_receiver_prefix_rest(rest, receiver)
        } else {
            lua_config_receiver_prefix_rest(statement, receiver)
        };
        let Some(after_receiver) = after_receiver else {
            continue;
        };
        let Some(after_field) = lua_config_field_access_rest_from_query_with_static_key(
            source,
            after_receiver,
            "hyperlink_rules",
            start,
        ) else {
            continue;
        };
        let after_field = lua_trim_start_comments(after_field)?;
        let Some(value) = after_field.strip_prefix('=') else {
            continue;
        };
        if lua_wezterm_default_hyperlink_rules_value_from_query_with_static_source(
            source,
            lua_trim_start_comments(value)?,
            start,
        ) {
            return Some(true);
        }
    }

    Some(false)
}

fn apply_config_hyperlink_rule_assignment_at_start(
    rules: &mut Vec<NativeHyperlinkRule>,
    source: &str,
    start: usize,
    receiver: &str,
) -> Option<bool> {
    if let Some(insert) =
        lua_config_table_insert_append_value_from_query(source, start, receiver, "hyperlink_rules")
    {
        apply_hyperlink_rule_insert(rules, source, &insert, start)?;
        return Some(true);
    }
    if let Some(assignment) = lua_config_table_index_or_append_assignment_from_query(
        source,
        start,
        receiver,
        "hyperlink_rules",
    ) {
        apply_hyperlink_rule_index_or_append_assignment(rules, source, &assignment, start)?;
        return Some(true);
    }
    if let Some(assignment) = lua_config_table_indexed_field_assignment_from_query(
        source,
        start,
        receiver,
        "hyperlink_rules",
    ) {
        return apply_hyperlink_rule_indexed_field_assignment(rules, source, &assignment, start);
    }
    Some(false)
}

fn apply_hyperlink_rule_insert(
    rules: &mut Vec<NativeHyperlinkRule>,
    source: &str,
    insert: &LuaTableInsertValue,
    max_start: usize,
) -> Option<()> {
    let rule = native_hyperlink_rule_lua_table_from_query(source, &insert.value, max_start)?;
    if let Some(position) = insert.position {
        let index = position.saturating_sub(1).min(rules.len());
        rules.insert(index, rule);
    } else {
        rules.push(rule);
    }
    Some(())
}

fn apply_hyperlink_rule_index_or_append_assignment(
    rules: &mut Vec<NativeHyperlinkRule>,
    source: &str,
    assignment: &LuaTableIndexOrAppendAssignment<String>,
    max_start: usize,
) -> Option<()> {
    let rule = native_hyperlink_rule_lua_table_from_query(source, &assignment.value, max_start)?;
    if let Some(position) = assignment.index {
        let index = position.saturating_sub(1);
        if index < rules.len() {
            rules[index] = rule;
        } else {
            rules.push(rule);
        }
    } else {
        rules.push(rule);
    }
    Some(())
}

fn apply_hyperlink_rule_indexed_field_assignment(
    rules: &mut [NativeHyperlinkRule],
    source: &str,
    assignment: &LuaTableIndexedFieldAssignment<'_>,
    max_start: usize,
) -> Option<bool> {
    let rule = rules.get_mut(assignment.index.checked_sub(1)?)?;
    match assignment.key.as_str() {
        "regex" => {
            rule.regex =
                lua_static_string_value_before_offset(source, assignment.value, max_start)?;
        }
        "format" => {
            rule.format =
                lua_static_string_value_before_offset(source, assignment.value, max_start)?;
        }
        "highlight" => {
            let value = lua_static_number_assignment_value_before_offset_from_query(
                source,
                assignment.value,
                max_start,
                lua_unsigned_integer_literal_from_query,
            )?;
            rule.highlight = value.parse().ok()?;
        }
        _ => return Some(false),
    }
    Some(true)
}

fn lua_config_hyperlink_rules_static_value_variable_before_offset(
    source: &str,
    max_start: usize,
) -> Option<&str> {
    if let Some(table) = lua_config_static_return_table_from_query(source)
        && lua_source_slice_start_offset(source, table) == Some(max_start)
        && let Some(variable) =
            lua_config_table_hyperlink_rules_static_value_variable(table, source, max_start)
    {
        return Some(variable);
    }

    let receiver = lua_config_static_return_identifier_from_query(source).unwrap_or("config");
    if let Some(returned_table) =
        lua_static_table_variable_assignment_before_offset_from_query(source, receiver, max_start)
            .or_else(|| {
                lua_static_table_variable_assignment_at_offset_from_query(
                    source, receiver, max_start,
                )
            })
        && let Some(variable) = lua_config_table_hyperlink_rules_static_value_variable(
            returned_table
                .trim()
                .strip_prefix('{')?
                .strip_suffix('}')?
                .trim(),
            source,
            max_start,
        )
    {
        return Some(variable);
    }

    None
}

fn lua_config_table_hyperlink_rules_static_value_variable<'a>(
    table: &'a str,
    source: &str,
    max_start: usize,
) -> Option<&'a str> {
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
        if key != "hyperlink_rules" {
            continue;
        }
        let value = lua_trim_start_comments(value)?;
        let variable = lua_identifier_literal_from_query(value)?;
        let rest = value.get(variable.len()..)?;
        if lua_static_identifier_value_rest_is_statement_end(rest) {
            return Some(variable);
        }
    }

    None
}

fn lua_static_table_variable_assignment_at_offset_from_query<'a>(
    source: &'a str,
    variable: &str,
    start: usize,
) -> Option<&'a str> {
    let rest = if lua_source_keyword_at(source, start, "local") {
        lua_trim_start_comments(source.get(start + "local".len()..)?)?
    } else {
        source.get(start..)?
    };

    lua_static_table_variable_assignment_table_from_query(rest, variable)
}

fn lua_config_table_hyperlink_rules_extends_default_rules(
    source: &str,
    table: &str,
    max_start: usize,
) -> Option<bool> {
    let mut extends_default = false;
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
        if key == "hyperlink_rules" {
            extends_default =
                lua_wezterm_default_hyperlink_rules_value_from_query_with_static_source(
                    source,
                    lua_trim_start_comments(value)?,
                    max_start,
                );
        }
    }

    Some(extends_default)
}

fn lua_wezterm_default_hyperlink_rules_value_from_query_with_static_source(
    source: &str,
    query: &str,
    max_start: usize,
) -> bool {
    if lua_wezterm_default_hyperlink_rules_value_from_query(query) {
        return true;
    }

    lua_static_expression_assignment_value_before_offset_from_query(source, query, max_start)
        .is_some_and(lua_wezterm_default_hyperlink_rules_value_from_query)
}

fn lua_wezterm_default_hyperlink_rules_value_from_query(query: &str) -> bool {
    let query = query.trim_start();
    let Some(rest) = query.strip_prefix("wezterm") else {
        return false;
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return false;
    }
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix('.') else {
        return false;
    };
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("default_hyperlink_rules") else {
        return false;
    };
    if rest.chars().next().is_some_and(is_lua_identifier_character) {
        return false;
    }
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix('(') else {
        return false;
    };
    let Some(rest) = lua_trim_start_comments(rest) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(')') else {
        return false;
    };
    lua_static_identifier_value_rest_is_statement_end(rest)
}

fn native_hyperlink_rule_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<NativeHyperlinkRule> {
    let resolved_value;
    let value = if value.trim_start().starts_with('{') {
        value
    } else {
        resolved_value = lua_table_insert_value_table_string_from_query(source, value, max_start)?;
        resolved_value.as_str()
    };
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut regex = None;
    let mut format = None;
    let mut highlight = 0usize;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "regex" => {
                regex = Some(lua_static_string_value_before_offset(
                    source, value, max_start,
                )?);
            }
            "format" => {
                format = Some(lua_static_string_value_before_offset(
                    source, value, max_start,
                )?);
            }
            "highlight" => {
                let value = lua_static_number_assignment_value_before_offset_from_query(
                    source,
                    value,
                    max_start,
                    lua_unsigned_integer_literal_from_query,
                )?;
                highlight = value.parse().ok()?;
            }
            _ => {}
        }
    }

    Some(NativeHyperlinkRule {
        regex: regex?,
        format: format?,
        highlight,
    })
}

fn lua_static_string_value_before_offset(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<String> {
    if let Some(value) = lua_quoted_string_literal_from_query(value)
        .or_else(|| lua_long_bracket_literal_from_query(value))
    {
        return parse_maybe_quoted_query_text(value);
    }

    lua_static_string_assignment_value_before_offset_from_query(source, value, max_start)
        .and_then(parse_maybe_quoted_query_text)
}

#[allow(dead_code)]
fn native_line_height_from_ratio(ratio: f32) -> Option<NativeLineHeight> {
    native_ratio_to_per_mille(ratio).map(NativeLineHeight::from_per_mille)
}

fn default_freetype_load_flags_for_dpi(dpi: u32) -> NativeFreetypeLoadFlags {
    if dpi >= FREETYPE_LOAD_FLAGS_NO_HINTING_DPI_THRESHOLD {
        NativeFreetypeLoadFlags::NO_HINTING
    } else {
        NativeFreetypeLoadFlags::DEFAULT
    }
}

#[allow(dead_code)]
fn native_hsb_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: Option<usize>,
) -> Option<NativeInactivePaneHsb> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = max_start.map(|max_start| LuaStaticSource { source, max_start });
    let mut hue = None;
    let mut saturation = None;
    let mut brightness = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = lua_static_number_assignment_value_from_query(
            source,
            value.trim(),
            lua_unsigned_number_literal_from_query,
        )?
        .parse::<f32>()
        .ok()?;
        match key.as_str() {
            "hue" => hue = Some(native_hsb_multiplier_from_ratio(value)?),
            "saturation" => saturation = Some(native_hsb_multiplier_from_ratio(value)?),
            "brightness" => brightness = Some(native_hsb_multiplier_from_ratio(value)?),
            _ => return None,
        }
    }

    Some(NativeInactivePaneHsb {
        hue: hue.unwrap_or(NativeHsbMultiplier::ONE),
        saturation: saturation.unwrap_or(NativeHsbMultiplier::ONE),
        brightness: brightness.unwrap_or(NativeHsbMultiplier::ONE),
    })
}

#[allow(dead_code)]
fn native_window_background_gradient_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: usize,
) -> Option<NativeWindowBackgroundGradient> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut orientation = NativeWindowBackgroundGradientOrientation::Horizontal;
    let mut interpolation = NativeWindowBackgroundGradientInterpolation::Linear;
    let mut blend = NativeWindowBackgroundGradientBlend::Rgb;
    let mut noise = None;
    let mut segment_size = None;
    let mut segment_smoothness_millis = 0;
    let mut preset = None;
    let mut colors = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "orientation" => {
                orientation = NativeWindowBackgroundGradientOrientation::parse_lua_value(
                    source, value, max_start,
                )?;
            }
            "interpolation" => {
                let value = lua_static_string_assignment_value_from_query(source, value)
                    .and_then(parse_maybe_quoted_query_text)?;
                interpolation = NativeWindowBackgroundGradientInterpolation::parse(&value)?;
            }
            "blend" => {
                let value = lua_static_string_assignment_value_from_query(source, value)
                    .and_then(parse_maybe_quoted_query_text)?;
                blend = NativeWindowBackgroundGradientBlend::parse(&value)?;
            }
            "noise" => {
                let value = lua_static_unsigned_u32_value_before_offset_from_query(
                    source, value, max_start,
                )?;
                noise = Some(usize::try_from(value).ok()?);
            }
            "segment_size" => {
                let size = lua_static_unsigned_u32_value_before_offset_from_query(
                    source, value, max_start,
                )?;
                let size = usize::try_from(size).ok()?;
                if size == 0 {
                    return None;
                }
                segment_size = Some(size);
            }
            "segment_smoothness" => {
                let smoothness = parse_maybe_static_query_f64(static_source, value)?;
                segment_smoothness_millis =
                    native_gradient_unit_interval_millis_from_f64(smoothness)?;
            }
            "colors" => {
                let parsed_colors =
                    split_lua_gradient_color_array_with_static_source(static_source, value)?
                        .into_iter()
                        .map(|color| {
                            lua_opaque_color_from_query_with_static_source(static_source, &color)
                        })
                        .collect::<Option<Vec<_>>>()?;
                if parsed_colors.len() < 2 {
                    return None;
                }
                colors = Some(parsed_colors);
            }
            "preset" => {
                let value = lua_static_string_assignment_value_from_query(source, value)
                    .and_then(parse_maybe_quoted_query_text)?;
                preset = Some(NativeWindowBackgroundGradientPreset::parse(&value)?);
            }
            _ => {}
        }
    }

    let colors = colors.unwrap_or_default();
    if colors.len() < 2 && preset.is_none() {
        return None;
    }

    Some(NativeWindowBackgroundGradient {
        orientation,
        interpolation,
        blend,
        noise,
        segment: segment_size.map(|size| NativeWindowBackgroundGradientSegment {
            size,
            smoothness_millis: segment_smoothness_millis,
        }),
        preset,
        opacity_alpha: u8::MAX,
        blend_with_background_color: false,
        hsb: native_identity_hsb(),
        colors,
    })
}

#[derive(Clone)]
enum NativeBackgroundLayer {
    Color(Color),
    Gradient(NativeWindowBackgroundGradient),
    Image(NativeWindowBackgroundImage),
    Images(Vec<NativeWindowBackgroundImage>),
    VisualLayers(Vec<NativeWindowBackgroundVisualLayer>),
    ColorAndGradient {
        color: Color,
        gradient: NativeWindowBackgroundGradient,
    },
    ColorAndImages {
        color: Color,
        images: Vec<NativeWindowBackgroundImage>,
    },
    ColorAndVisualLayers {
        color: Color,
        layers: Vec<NativeWindowBackgroundVisualLayer>,
    },
}

fn apply_lua_background_table_overrides(
    source: &str,
    value: &str,
    max_start: usize,
    overrides: &mut NativeConfigSnapshot,
) -> Option<bool> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let layers = native_background_layers_lua_table_from_query(source, table, max_start)?;
    let mut background = overrides.background.take().unwrap_or_default();
    background.extend(native_background_visual_layers_from_layers(&layers)?);
    overrides.background = Some(background);

    match native_background_lua_table_from_layers(layers)? {
        NativeBackgroundLayer::Color(color) => {
            if let Some(gradient) = overrides.window_background_gradient.take() {
                overrides.window_background_gradient =
                    Some(compose_lua_background_color_over_gradient(gradient, color));
            } else if let Some(images) = overrides.window_background_images.take() {
                if images.is_empty() {
                    overrides.background_color = Some(color);
                } else {
                    apply_native_background_visual_layers_override(
                        images
                            .into_iter()
                            .map(NativeWindowBackgroundVisualLayer::Image)
                            .chain(std::iter::once(NativeWindowBackgroundVisualLayer::Color(
                                color,
                            )))
                            .collect(),
                        overrides,
                    );
                }
            } else {
                overrides.background_color = Some(color);
            }
        }
        NativeBackgroundLayer::Gradient(gradient) => {
            overrides.window_background_gradient = Some(gradient);
        }
        NativeBackgroundLayer::Image(image) => {
            overrides.window_background_images = Some(vec![image]);
        }
        NativeBackgroundLayer::Images(images) => {
            overrides.window_background_images = Some(images);
        }
        NativeBackgroundLayer::VisualLayers(layers) => {
            apply_native_background_visual_layers_override(layers, overrides);
        }
        NativeBackgroundLayer::ColorAndGradient { color, gradient } => {
            overrides.background_color = Some(color);
            overrides.window_background_gradient = Some(gradient);
        }
        NativeBackgroundLayer::ColorAndImages { color, images } => {
            overrides.background_color = Some(color);
            overrides.window_background_images = Some(images);
        }
        NativeBackgroundLayer::ColorAndVisualLayers { color, layers } => {
            overrides.background_color = Some(color);
            apply_native_background_visual_layers_override(layers, overrides);
        }
    }
    Some(true)
}

fn apply_native_background_visual_layers_override(
    layers: Vec<NativeWindowBackgroundVisualLayer>,
    overrides: &mut NativeConfigSnapshot,
) {
    overrides.window_background_gradient = layers.iter().find_map(|layer| match layer {
        NativeWindowBackgroundVisualLayer::Gradient(gradient) => Some(gradient.clone()),
        NativeWindowBackgroundVisualLayer::Color(_)
        | NativeWindowBackgroundVisualLayer::Image(_) => None,
    });
    let images = layers
        .iter()
        .filter_map(|layer| match layer {
            NativeWindowBackgroundVisualLayer::Image(image) => Some(image.clone()),
            NativeWindowBackgroundVisualLayer::Color(_)
            | NativeWindowBackgroundVisualLayer::Gradient(_) => None,
        })
        .collect::<Vec<_>>();
    if !images.is_empty() {
        overrides.window_background_images = Some(images);
    }
    overrides.window_background_layers = Some(layers);
}

fn native_background_lua_table_from_layers(
    layers: Vec<NativeBackgroundLayer>,
) -> Option<NativeBackgroundLayer> {
    let mut color = None;
    let mut visual_layers = Vec::new();
    for layer in layers {
        match layer {
            NativeBackgroundLayer::Color(layer) if visual_layers.is_empty() => {
                color = Some(match color {
                    Some(color) => compose_lua_background_color_layers(color, layer),
                    None => layer,
                });
            }
            NativeBackgroundLayer::Gradient(layer) => {
                visual_layers.push(NativeWindowBackgroundVisualLayer::Gradient(layer));
            }
            NativeBackgroundLayer::Image(layer) => {
                visual_layers.push(NativeWindowBackgroundVisualLayer::Image(layer));
            }
            NativeBackgroundLayer::Color(layer) => {
                visual_layers.push(NativeWindowBackgroundVisualLayer::Color(layer));
            }
            NativeBackgroundLayer::Images(_)
            | NativeBackgroundLayer::VisualLayers(_)
            | NativeBackgroundLayer::ColorAndGradient { .. }
            | NativeBackgroundLayer::ColorAndImages { .. }
            | NativeBackgroundLayer::ColorAndVisualLayers { .. } => return None,
        }
    }

    if visual_layers.len() == 1 {
        match visual_layers.pop()? {
            NativeWindowBackgroundVisualLayer::Color(color) => {
                return Some(NativeBackgroundLayer::Color(color));
            }
            NativeWindowBackgroundVisualLayer::Gradient(gradient) => {
                return if let Some(color) = color {
                    Some(NativeBackgroundLayer::ColorAndGradient {
                        color,
                        gradient: compose_lua_background_color_below_gradient(color, gradient),
                    })
                } else {
                    Some(NativeBackgroundLayer::Gradient(gradient))
                };
            }
            NativeWindowBackgroundVisualLayer::Image(image) => {
                return if let Some(color) = color {
                    Some(NativeBackgroundLayer::ColorAndImages {
                        color,
                        images: vec![image],
                    })
                } else {
                    Some(NativeBackgroundLayer::Image(image))
                };
            }
        }
    }

    if !visual_layers.is_empty()
        && visual_layers
            .iter()
            .all(|layer| matches!(layer, NativeWindowBackgroundVisualLayer::Image(_)))
    {
        let images = visual_layers
            .into_iter()
            .filter_map(|layer| match layer {
                NativeWindowBackgroundVisualLayer::Image(image) => Some(image),
                NativeWindowBackgroundVisualLayer::Color(_)
                | NativeWindowBackgroundVisualLayer::Gradient(_) => None,
            })
            .collect::<Vec<_>>();
        return if let Some(color) = color {
            Some(NativeBackgroundLayer::ColorAndImages { color, images })
        } else {
            Some(NativeBackgroundLayer::Images(images))
        };
    }

    if visual_layers.is_empty() {
        color.map(NativeBackgroundLayer::Color)
    } else if let Some(color) = color {
        Some(NativeBackgroundLayer::ColorAndVisualLayers {
            color,
            layers: visual_layers,
        })
    } else {
        Some(NativeBackgroundLayer::VisualLayers(visual_layers))
    }
}

fn native_background_visual_layers_from_layers(
    layers: &[NativeBackgroundLayer],
) -> Option<Vec<NativeWindowBackgroundVisualLayer>> {
    layers
        .iter()
        .map(|layer| match layer {
            NativeBackgroundLayer::Color(color) => {
                Some(NativeWindowBackgroundVisualLayer::Color(*color))
            }
            NativeBackgroundLayer::Gradient(gradient) => Some(
                NativeWindowBackgroundVisualLayer::Gradient(gradient.clone()),
            ),
            NativeBackgroundLayer::Image(image) => {
                Some(NativeWindowBackgroundVisualLayer::Image(image.clone()))
            }
            NativeBackgroundLayer::Images(_)
            | NativeBackgroundLayer::VisualLayers(_)
            | NativeBackgroundLayer::ColorAndGradient { .. }
            | NativeBackgroundLayer::ColorAndImages { .. }
            | NativeBackgroundLayer::ColorAndVisualLayers { .. } => None,
        })
        .collect()
}

fn native_background_layers_lua_table_from_query(
    source: &str,
    table: &str,
    max_start: usize,
) -> Option<Vec<NativeBackgroundLayer>> {
    let mut layers = Vec::new();
    let mut indexed_layers = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !layers.is_empty() || index == 0 || indexed_layers.contains_key(&index) {
                return None;
            }
            indexed_layers.insert(
                index,
                native_background_layer_lua_value_from_query(source, value.trim(), max_start)?,
            );
            continue;
        }

        if !indexed_layers.is_empty() {
            return None;
        }
        layers.push(native_background_layer_lua_value_from_query(
            source, field, max_start,
        )?);
    }

    if !indexed_layers.is_empty() {
        return (1..=indexed_layers.len())
            .map(|index| indexed_layers.remove(&index))
            .collect();
    }

    Some(layers)
}

fn native_background_layer_lua_value_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<NativeBackgroundLayer> {
    let layer = lua_background_layer_table_from_query(source, value, max_start)?;
    native_background_layer_lua_table_from_query(source, &layer, max_start)
}

fn compose_lua_background_color_layers(background: Color, foreground: Color) -> Color {
    let background = color_to_rgba(background, DEFAULT_RENDER_BACKGROUND_RGBA);
    let foreground = color_to_rgba(foreground, DEFAULT_RENDER_BACKGROUND_RGBA);
    let foreground_alpha = u32::from(foreground[3]);
    let background_alpha = u32::from(background[3]);
    let inverse_alpha = u32::from(u8::MAX) - foreground_alpha;
    let alpha =
        foreground_alpha + background_alpha.saturating_mul(inverse_alpha) / u32::from(u8::MAX);
    if alpha == 0 {
        return Color::Rgba(0, 0, 0, 0);
    }

    let channel = |index: usize| {
        let foreground_weight = u32::from(foreground[index]).saturating_mul(foreground_alpha);
        let background_weight = u32::from(background[index])
            .saturating_mul(background_alpha)
            .saturating_mul(inverse_alpha)
            / u32::from(u8::MAX);
        let value = (foreground_weight + background_weight) / alpha;
        u8::try_from(value).unwrap_or(u8::MAX)
    };

    rgba_to_color([
        channel(0),
        channel(1),
        channel(2),
        u8::try_from(alpha.min(u32::from(u8::MAX))).unwrap_or(u8::MAX),
    ])
}

fn compose_lua_background_color_below_gradient(
    color: Color,
    mut gradient: NativeWindowBackgroundGradient,
) -> NativeWindowBackgroundGradient {
    if gradient.colors.is_empty() {
        gradient.blend_with_background_color = true;
        return gradient;
    }
    gradient.colors = gradient
        .colors
        .into_iter()
        .map(|gradient_color| compose_lua_background_color_layers(color, gradient_color))
        .collect();
    gradient
}

fn compose_lua_background_color_over_gradient(
    mut gradient: NativeWindowBackgroundGradient,
    color: Color,
) -> NativeWindowBackgroundGradient {
    if gradient.colors.is_empty() {
        return gradient;
    }
    gradient.colors = gradient
        .colors
        .into_iter()
        .map(|gradient_color| compose_lua_background_color_layers(gradient_color, color))
        .collect();
    gradient
}

fn lua_background_layer_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<String> {
    let value = value.trim();
    if value.starts_with('{') {
        return Some(lua_braced_table_literal_from_query(value)?.to_owned());
    }

    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_table_variable_assignment_before_offset_from_query(source, variable, max_start)
        .map(str::to_owned)
}

fn native_background_layer_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<NativeBackgroundLayer> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let static_source = Some(LuaStaticSource { source, max_start });
    let mut source_value = None;
    let mut hsb = native_identity_hsb();
    let mut opacity = 1.0;
    let mut attachment = RenderBackgroundImageAttachment::Fixed;
    let mut image_layout = NativeWindowBackgroundImageLayout::default();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "source" => {
                if source_value.is_some() {
                    return None;
                }
                source_value = Some(value);
            }
            "hsb" => {
                hsb = native_hsb_lua_value_from_query(source, value, max_start)?;
            }
            "opacity" => {
                opacity = parse_maybe_static_query_f64(static_source, value)?;
                if !opacity.is_finite() || opacity < 0.0 {
                    return None;
                }
            }
            "attachment" => {
                attachment =
                    native_background_image_attachment_lua_value_from_query(static_source, value)?;
            }
            "width" => {
                image_layout.width =
                    native_background_image_dimension_lua_value_from_query(static_source, value)?;
            }
            "height" => {
                image_layout.height =
                    native_background_image_dimension_lua_value_from_query(static_source, value)?;
            }
            "repeat_x" => {
                image_layout.repeat_x =
                    native_background_image_repeat_lua_value_from_query(static_source, value)?;
            }
            "repeat_y" => {
                image_layout.repeat_y =
                    native_background_image_repeat_lua_value_from_query(static_source, value)?;
            }
            "horizontal_align" => {
                image_layout.horizontal_align =
                    native_background_image_horizontal_align_lua_value_from_query(
                        static_source,
                        value,
                    )?;
            }
            "vertical_align" => {
                image_layout.vertical_align =
                    native_background_image_vertical_align_lua_value_from_query(
                        static_source,
                        value,
                    )?;
            }
            "horizontal_offset" => {
                image_layout.horizontal_offset =
                    native_background_image_length_lua_value_from_query(static_source, value)?;
            }
            "vertical_offset" => {
                image_layout.vertical_offset =
                    native_background_image_length_lua_value_from_query(static_source, value)?;
            }
            "repeat_x_size" => {
                image_layout.repeat_x_size = Some(
                    native_background_image_positive_length_lua_value_from_query(
                        static_source,
                        value,
                    )?,
                );
            }
            "repeat_y_size" => {
                image_layout.repeat_y_size = Some(
                    native_background_image_positive_length_lua_value_from_query(
                        static_source,
                        value,
                    )?,
                );
            }
            _ => return None,
        }
    }

    native_background_source_lua_table_from_query(
        static_source,
        source_value?,
        hsb,
        opacity,
        attachment,
        image_layout,
    )
}

fn native_background_image_dimension_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<RenderBackgroundImageDimension> {
    if let Some(pixels) = parse_maybe_static_query_usize(static_source, value.trim()) {
        return Some(RenderBackgroundImageDimension::Pixels(
            u32::try_from(pixels).ok()?,
        ));
    }

    let value = parse_maybe_static_query_text(static_source, value.trim())?;
    match value.as_str() {
        "Cover" => Some(RenderBackgroundImageDimension::Cover),
        "Contain" => Some(RenderBackgroundImageDimension::Contain),
        _ => {
            if let Some(percent) = value.strip_suffix('%') {
                return Some(RenderBackgroundImageDimension::Percent(
                    parse_background_image_percent_basis_points(percent)?,
                ));
            }
            if let Some(cells) = value.strip_suffix("cell") {
                return Some(RenderBackgroundImageDimension::Cells(
                    cells.parse::<u32>().ok()?,
                ));
            }
            let pixels = value.strip_suffix("px").unwrap_or(&value);
            Some(RenderBackgroundImageDimension::Pixels(pixels.parse().ok()?))
        }
    }
}

fn native_background_image_length_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<RenderBackgroundImageLength> {
    if let Some(pixels) = parse_maybe_static_query_isize(static_source, value.trim()) {
        return Some(RenderBackgroundImageLength::Pixels(
            i32::try_from(pixels).ok()?,
        ));
    }

    let value = parse_maybe_static_query_text(static_source, value.trim())?;
    if let Some(percent) = value.strip_suffix('%') {
        return Some(RenderBackgroundImageLength::Percent(
            parse_background_image_signed_percent_basis_points(percent)?,
        ));
    }
    if let Some(cells) = value.strip_suffix("cell") {
        return Some(RenderBackgroundImageLength::Cells(cells.parse().ok()?));
    }
    let pixels = value.strip_suffix("px").unwrap_or(&value);
    Some(RenderBackgroundImageLength::Pixels(pixels.parse().ok()?))
}

fn native_background_image_positive_length_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<RenderBackgroundImageLength> {
    let length = native_background_image_length_lua_value_from_query(static_source, value)?;
    match length {
        RenderBackgroundImageLength::Pixels(value)
        | RenderBackgroundImageLength::Percent(value)
        | RenderBackgroundImageLength::Cells(value)
            if value <= 0 =>
        {
            None
        }
        _ => Some(length),
    }
}

fn parse_background_image_percent_basis_points(value: &str) -> Option<u32> {
    let basis_points = parse_background_image_signed_percent_basis_points(value)?;
    if basis_points < 0 {
        return None;
    }
    u32::try_from(basis_points).ok()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "finite percentages are rounded and bounded to i32 before conversion"
)]
fn parse_background_image_signed_percent_basis_points(value: &str) -> Option<i32> {
    let percent: f64 = parse_single_query_value(value)?.parse().ok()?;
    if !percent.is_finite() {
        return None;
    }
    let basis_points = (percent * 100.0).round();
    if basis_points < f64::from(i32::MIN) || basis_points > f64::from(i32::MAX) {
        return None;
    }
    Some(basis_points as i32)
}

fn native_background_image_repeat_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<RenderBackgroundImageRepeat> {
    match parse_maybe_static_query_text(static_source, value.trim())?.as_str() {
        "Repeat" => Some(RenderBackgroundImageRepeat::Repeat),
        "Mirror" => Some(RenderBackgroundImageRepeat::Mirror),
        "NoRepeat" => Some(RenderBackgroundImageRepeat::NoRepeat),
        _ => None,
    }
}

fn native_background_image_horizontal_align_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<RenderBackgroundImageHorizontalAlign> {
    match parse_maybe_static_query_text(static_source, value.trim())?.as_str() {
        "Left" => Some(RenderBackgroundImageHorizontalAlign::Left),
        "Center" => Some(RenderBackgroundImageHorizontalAlign::Center),
        "Right" => Some(RenderBackgroundImageHorizontalAlign::Right),
        _ => None,
    }
}

fn native_background_image_vertical_align_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<RenderBackgroundImageVerticalAlign> {
    match parse_maybe_static_query_text(static_source, value.trim())?.as_str() {
        "Top" => Some(RenderBackgroundImageVerticalAlign::Top),
        "Middle" => Some(RenderBackgroundImageVerticalAlign::Middle),
        "Bottom" => Some(RenderBackgroundImageVerticalAlign::Bottom),
        _ => None,
    }
}

fn native_background_image_attachment_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<RenderBackgroundImageAttachment> {
    if let Some(value) = parse_maybe_static_query_text(static_source, value.trim()) {
        match value.as_str() {
            "Fixed" => return Some(RenderBackgroundImageAttachment::Fixed),
            "Scroll" => return Some(RenderBackgroundImageAttachment::Scroll),
            _ => {}
        }
    }

    let value = lua_background_attachment_table_from_query(static_source, value)?;
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut fields = split_lua_table_top_level_fields(table)?
        .into_iter()
        .map(str::trim)
        .filter(|field| !field.is_empty());
    let field = fields.next()?;
    if fields.next().is_some() {
        return None;
    }

    let (key, value) = split_lua_table_assignment_from_field(field)?;
    let key = split_lua_table_key_from_query(key.trim())?;
    match key.as_str() {
        "Parallax" => Some(RenderBackgroundImageAttachment::Parallax {
            factor_millis: native_background_parallax_factor_lua_value_from_query(
                static_source,
                value,
            )?,
        }),
        _ => None,
    }
}

fn lua_background_attachment_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    let value = value.trim();
    if value.starts_with('{') {
        return Some(lua_braced_table_literal_from_query(value)?.to_owned());
    }

    let static_source = static_source?;
    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_table_variable_assignment_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )
    .map(str::to_owned)
}

#[allow(clippy::cast_possible_truncation)]
fn native_background_parallax_factor_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<i32> {
    let factor = parse_maybe_static_query_f64(static_source, value.trim())?;
    if !factor.is_finite()
        || factor < f64::from(i32::MIN) / 1_000.0
        || factor > f64::from(i32::MAX) / 1_000.0
    {
        return None;
    }
    Some((factor * 1_000.0).round() as i32)
}

fn native_hsb_lua_value_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<NativeInactivePaneHsb> {
    let value = value.trim();
    if value.starts_with('{') {
        return native_hsb_lua_table_from_query(source, value, Some(max_start));
    }

    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    let value =
        lua_static_table_variable_assignment_before_offset_from_query(source, variable, max_start)?;
    native_hsb_lua_table_from_query(source, value, lua_source_slice_start_offset(source, value))
}

fn native_background_source_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    hsb: NativeInactivePaneHsb,
    opacity: f64,
    attachment: RenderBackgroundImageAttachment,
    image_layout: NativeWindowBackgroundImageLayout,
) -> Option<NativeBackgroundLayer> {
    let value = lua_background_source_table_from_query(static_source, value)?;
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut fields = split_lua_table_top_level_fields(table)?
        .into_iter()
        .map(str::trim)
        .filter(|field| !field.is_empty());
    let field = fields.next()?;
    if fields.next().is_some() {
        return None;
    }
    let (key, value) = split_lua_table_assignment_from_field(field)?;
    let key = split_lua_table_key_from_query(key.trim())?;
    match key.as_str() {
        "Color" => {
            let color = parse_maybe_static_query_text(static_source, value.trim())?;
            Some(NativeBackgroundLayer::Color(
                lua_background_color_with_hsb_and_opacity(
                    lua_color_from_query_with_static_source(static_source, &color)?,
                    hsb,
                    opacity,
                ),
            ))
        }
        "Gradient" => {
            let static_source = static_source?;
            let gradient = lua_background_source_gradient_table_from_query(static_source, value)?;
            let mut gradient = native_window_background_gradient_lua_table_from_query(
                static_source.source,
                &gradient,
                static_source.max_start,
            )?;
            let opacity_alpha = opacity_alpha(opacity);
            if hsb != native_identity_hsb() {
                if gradient.colors.is_empty() {
                    gradient.hsb = hsb;
                    gradient.opacity_alpha = opacity_alpha;
                } else {
                    gradient.colors = gradient
                        .colors
                        .into_iter()
                        .map(|color| lua_background_color_with_hsb_and_opacity(color, hsb, opacity))
                        .collect();
                }
            } else if opacity_alpha != u8::MAX {
                if gradient.colors.is_empty() {
                    gradient.opacity_alpha = opacity_alpha;
                } else {
                    gradient.colors = gradient
                        .colors
                        .into_iter()
                        .map(|color| lua_background_color_with_hsb_and_opacity(color, hsb, opacity))
                        .collect();
                }
            }
            Some(NativeBackgroundLayer::Gradient(gradient))
        }
        "File" => {
            let file =
                native_background_file_source_lua_value_from_query(static_source, value.trim())?;
            let data = fs::read(Path::new(&file.path)).ok()?;
            Some(NativeBackgroundLayer::Image(NativeWindowBackgroundImage {
                data,
                opacity_alpha: opacity_alpha(opacity),
                hsb,
                animation_speed_millis: file.animation_speed_millis,
                attachment,
                layout: image_layout,
            }))
        }
        _ => None,
    }
}

struct NativeBackgroundFileSource {
    path: String,
    animation_speed_millis: u32,
}

fn native_background_file_source_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeBackgroundFileSource> {
    if !value.trim().starts_with('{')
        && let Some(path) = parse_maybe_static_query_text(static_source, value.trim())
    {
        return Some(NativeBackgroundFileSource {
            path,
            animation_speed_millis: 1_000,
        });
    }

    let value = lua_background_source_file_table_from_query(static_source, value)?;
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut path = None;
    let mut animation_speed_millis = 1_000;
    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        match key.as_str() {
            "path" => {
                if path.is_some() {
                    return None;
                }
                path = Some(parse_maybe_static_query_text(static_source, value.trim())?);
            }
            "speed" => {
                animation_speed_millis =
                    native_background_file_speed_lua_value_from_query(static_source, value)?;
            }
            _ => return None,
        }
    }

    Some(NativeBackgroundFileSource {
        path: path?,
        animation_speed_millis,
    })
}

fn lua_background_source_file_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    let value = value.trim();
    if value.starts_with('{') {
        return Some(lua_braced_table_literal_from_query(value)?.to_owned());
    }

    let static_source = static_source?;
    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_table_variable_assignment_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )
    .map(str::to_owned)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn native_background_file_speed_lua_value_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<u32> {
    let speed = parse_maybe_static_query_f64(static_source, value.trim())?;
    if !speed.is_finite() || speed < 0.0 || speed > f64::from(u32::MAX) / 1_000.0 {
        return None;
    }
    Some((speed * 1_000.0).round() as u32)
}

fn lua_background_source_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<String> {
    let value = value.trim();
    if value.starts_with('{') {
        return Some(lua_braced_table_literal_from_query(value)?.to_owned());
    }

    let static_source = static_source?;
    let variable = lua_identifier_literal_from_query(value)?;
    let rest = value.get(variable.len()..)?;
    if !lua_static_identifier_value_rest_is_statement_end(rest) {
        return None;
    }
    lua_static_table_variable_assignment_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )
    .map(str::to_owned)
}

fn native_identity_hsb() -> NativeInactivePaneHsb {
    NativeInactivePaneHsb {
        hue: NativeHsbMultiplier::ONE,
        saturation: NativeHsbMultiplier::ONE,
        brightness: NativeHsbMultiplier::ONE,
    }
}

fn lua_background_source_gradient_table_from_query(
    static_source: LuaStaticSource<'_>,
    value: &str,
) -> Option<String> {
    let value = value.trim();
    if value.starts_with('{') {
        return Some(lua_braced_table_literal_from_query(value)?.to_owned());
    }

    let variable = lua_identifier_literal_from_query(value)?;
    lua_static_table_variable_assignment_before_offset_from_query(
        static_source.source,
        variable,
        static_source.max_start,
    )
    .map(str::to_owned)
}

fn lua_background_color_with_hsb_and_opacity(
    color: Color,
    hsb: NativeInactivePaneHsb,
    opacity: f64,
) -> Color {
    let color = hsb_color(color, DEFAULT_RENDER_BACKGROUND_RGBA, hsb);
    let [red, green, blue, alpha] = color_to_rgba(color, DEFAULT_RENDER_BACKGROUND_RGBA);
    let opacity = f64::from(alpha) / f64::from(u8::MAX) * opacity.clamp(0.0, 1.0);
    let alpha = opacity_alpha(opacity);
    if alpha == u8::MAX {
        Color::Rgb(red, green, blue)
    } else {
        Color::Rgba(red, green, blue, alpha)
    }
}

#[allow(dead_code)]
fn native_daemon_options_lua_table_from_query(
    source: &str,
    value: &str,
) -> Option<NativeDaemonOptions> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut options = NativeDaemonOptions::default();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = lua_static_string_assignment_value_from_query(source, value.trim())
            .and_then(parse_maybe_quoted_query_text)?;
        match key.as_str() {
            "pid_file" => options.pid_file = Some(value),
            "stdout" => options.stdout = Some(value),
            "stderr" => options.stderr = Some(value),
            _ => return None,
        }
    }

    Some(options)
}

#[allow(dead_code)]
fn native_webgpu_preferred_adapter_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: Option<usize>,
) -> Option<NativeWebGpuPreferredAdapter> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut adapter = NativeWebGpuPreferredAdapter {
        backend: None,
        device: None,
        device_type: None,
        driver: None,
        driver_info: None,
        name: None,
        vendor: None,
    };
    let static_source = max_start.map(|max_start| LuaStaticSource { source, max_start });

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = value.trim();
        match key.as_str() {
            "backend" => {
                adapter.backend = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "device" => {
                adapter.device = Some(
                    lua_static_number_assignment_value_from_query(
                        source,
                        value,
                        lua_unsigned_integer_literal_from_query,
                    )?
                    .parse()
                    .ok()?,
                );
            }
            "device_type" => {
                adapter.device_type = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "driver" => {
                adapter.driver = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "driver_info" => {
                adapter.driver_info = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "name" => {
                adapter.name = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "vendor" => {
                adapter.vendor = Some(
                    lua_static_number_assignment_value_from_query(
                        source,
                        value,
                        lua_unsigned_integer_literal_from_query,
                    )?
                    .parse()
                    .ok()?,
                );
            }
            _ => return None,
        }
    }

    Some(adapter)
}

#[allow(dead_code)]
fn native_window_padding_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: Option<usize>,
) -> Option<NativeWindowPadding> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut padding = DEFAULT_WINDOW_PADDING;
    let static_source = max_start.map(|max_start| LuaStaticSource { source, max_start });

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = value.trim();
        let value = lua_static_string_assignment_value_from_query(source, value)
            .and_then(parse_maybe_quoted_query_text)
            .or_else(|| {
                lua_static_number_assignment_value_from_query(
                    source,
                    value,
                    lua_unsigned_number_literal_from_query,
                )
                .map(str::to_owned)
            })?;
        let dimension = NativeWindowPaddingDimension::parse(&value)?;

        match key.as_str() {
            "left" => padding.left = dimension,
            "right" => padding.right = dimension,
            "top" => padding.top = dimension,
            "bottom" => padding.bottom = dimension,
            _ => return None,
        }
    }

    Some(padding)
}

#[allow(dead_code)]
fn native_window_content_alignment_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: Option<usize>,
) -> Option<NativeWindowContentAlignment> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut alignment = DEFAULT_WINDOW_CONTENT_ALIGNMENT;
    let static_source = max_start.map(|max_start| LuaStaticSource { source, max_start });

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = lua_static_string_assignment_value_from_query(source, value.trim())?;
        let value = parse_maybe_quoted_query_text(value)?;

        match key.as_str() {
            "horizontal" => alignment.horizontal = NativeHorizontalContentAlignment::parse(&value)?,
            "vertical" => alignment.vertical = NativeVerticalContentAlignment::parse(&value)?,
            _ => return None,
        }
    }
    Some(alignment)
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn native_window_frame_appearance_lua_table_from_query(
    source: &str,
    value: &str,
    static_source: Option<LuaStaticSource<'_>>,
) -> Option<Option<NativeWindowFrameAppearance>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut appearance = NativeWindowFrameAppearance::default();
    let mut parsed = false;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let slot = match key.as_str() {
            "inactive_titlebar_bg" => &mut appearance.inactive_titlebar_bg,
            "active_titlebar_bg" => &mut appearance.active_titlebar_bg,
            "inactive_titlebar_fg" => &mut appearance.inactive_titlebar_fg,
            "active_titlebar_fg" => &mut appearance.active_titlebar_fg,
            "inactive_titlebar_border_bottom" => &mut appearance.inactive_titlebar_border_bottom,
            "active_titlebar_border_bottom" => &mut appearance.active_titlebar_border_bottom,
            "button_fg" => &mut appearance.button_fg,
            "button_bg" => &mut appearance.button_bg,
            "button_hover_fg" => &mut appearance.button_hover_fg,
            "button_hover_bg" => &mut appearance.button_hover_bg,
            "border_left_width" => {
                let value = lua_static_string_assignment_value_from_query(source, value.trim())
                    .or_else(|| {
                        lua_static_number_assignment_value_from_query(
                            source,
                            value,
                            lua_unsigned_number_literal_from_query,
                        )
                    })?;
                appearance.border_left_width = Some(NativeWindowPaddingDimension::parse(
                    &parse_maybe_quoted_query_text(value)?,
                )?);
                parsed = true;
                continue;
            }
            "border_right_width" => {
                let value = lua_static_string_assignment_value_from_query(source, value.trim())
                    .or_else(|| {
                        lua_static_number_assignment_value_from_query(
                            source,
                            value,
                            lua_unsigned_number_literal_from_query,
                        )
                    })?;
                appearance.border_right_width = Some(NativeWindowPaddingDimension::parse(
                    &parse_maybe_quoted_query_text(value)?,
                )?);
                parsed = true;
                continue;
            }
            "border_top_height" => {
                let value = lua_static_string_assignment_value_from_query(source, value.trim())
                    .or_else(|| {
                        lua_static_number_assignment_value_from_query(
                            source,
                            value,
                            lua_unsigned_number_literal_from_query,
                        )
                    })?;
                appearance.border_top_height = Some(NativeWindowPaddingDimension::parse(
                    &parse_maybe_quoted_query_text(value)?,
                )?);
                parsed = true;
                continue;
            }
            "border_bottom_height" => {
                let value = lua_static_string_assignment_value_from_query(source, value.trim())
                    .or_else(|| {
                        lua_static_number_assignment_value_from_query(
                            source,
                            value,
                            lua_unsigned_number_literal_from_query,
                        )
                    })?;
                appearance.border_bottom_height = Some(NativeWindowPaddingDimension::parse(
                    &parse_maybe_quoted_query_text(value)?,
                )?);
                parsed = true;
                continue;
            }
            "border_left_color" => &mut appearance.border_left_color,
            "border_right_color" => &mut appearance.border_right_color,
            "border_top_color" => &mut appearance.border_top_color,
            "border_bottom_color" => &mut appearance.border_bottom_color,
            "font" => {
                appearance.font = parse_wezterm_font_value(static_source, source, value.trim());
                parsed = true;
                continue;
            }
            "font_size" => {
                let value = lua_static_string_assignment_value_from_query(source, value.trim())
                    .or_else(|| {
                        lua_static_number_assignment_value_from_query(
                            source,
                            value,
                            lua_unsigned_number_literal_from_query,
                        )
                    })?;
                let value = parse_maybe_quoted_query_text(value)?;
                appearance.font_size =
                    Some(native_font_size_from_points(value.parse::<f32>().ok()?)?);
                parsed = true;
                continue;
            }
            _ => continue,
        };
        *slot = Some(native_window_frame_opaque_color_lua_value_from_query(
            source,
            value.trim(),
        )?);
        parsed = true;
    }

    Some(parsed.then_some(appearance))
}

fn native_window_frame_opaque_color_lua_value_from_query(
    source: &str,
    value: &str,
) -> Option<Color> {
    let mut color_max_start = lua_source_slice_start_offset(source, value);
    let value = if let Some(max_start) = color_max_start {
        if let Some(value) =
            lua_static_string_assignment_value_before_offset_from_query(source, value, max_start)
        {
            color_max_start = lua_source_slice_start_offset(source, value).or(color_max_start);
            value
        } else {
            value
        }
    } else {
        lua_static_string_assignment_value_from_query(source, value).unwrap_or(value)
    };
    let value = parse_maybe_quoted_query_text(value)?;
    lua_opaque_color_from_query_with_static_source(
        color_max_start.map(|max_start| LuaStaticSource { source, max_start }),
        &value,
    )
}

fn parse_wezterm_font_value(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
    value: &str,
) -> Option<String> {
    let value_source = lua_source_slice_start_offset(source, value)
        .map(|max_start| LuaStaticSource { source, max_start });
    let resolved_value = static_source.or(value_source).and_then(|static_source| {
        lua_static_wezterm_font_value_assignment_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
        .map(str::to_owned)
        .or_else(|| {
            lua_static_wezterm_font_alias_query_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
        .or_else(|| {
            lua_static_wezterm_font_call_query_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
    });
    let value = resolved_value.as_deref().unwrap_or(value);
    let normalized_value = static_source.or(value_source).and_then(|static_source| {
        lua_static_wezterm_font_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
        .or_else(|| {
            lua_static_wezterm_font_call_query_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
    });
    let value = normalized_value.as_deref().unwrap_or(value);

    lua_static_string_assignment_value_from_query(source, value)
        .and_then(parse_maybe_quoted_query_text)
        .or_else(|| parse_wezterm_font_table_family_value(static_source, source, value))
        .or_else(|| {
            let value = value.trim();
            let mut rest = lua_function_name_rest_from_query(value, "wezterm.font")?;
            if let Some(stripped) = rest.strip_prefix('(') {
                rest = stripped.trim_start();
            }
            let quote = rest.find(['\'', '"'])?;
            let literal = lua_quoted_string_literal_from_query(rest.get(quote..)?)?;
            parse_lua_quoted_query_text(literal)
        })
}

fn parse_wezterm_font_table_family_value(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
    value: &str,
) -> Option<String> {
    let table = wezterm_font_table_literal_from_query(value)?;
    let value = lua_table_field_value_from_query(table, "family")??;
    lua_static_string_assignment_value_from_query(source, value)
        .or_else(|| {
            let static_source = static_source?;
            lua_static_string_assignment_value_before_offset_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
        .and_then(parse_maybe_quoted_query_text)
}

fn parse_wezterm_font_families_value(source: &str, value: &str) -> Option<Vec<String>> {
    parse_wezterm_font_with_fallback_families_value(source, value)
        .or_else(|| parse_wezterm_font_value(None, source, value).map(|family| vec![family]))
}

fn parse_wezterm_font_config_value(source: &str, value: &str) -> Option<NativeFontConfig> {
    parse_wezterm_font_config_value_with_static_source(None, source, value)
}

fn parse_wezterm_font_config_value_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
    value: &str,
) -> Option<NativeFontConfig> {
    let value_source = lua_source_slice_start_offset(source, value)
        .map(|max_start| LuaStaticSource { source, max_start });
    let resolved_value = static_source.or(value_source).and_then(|static_source| {
        lua_static_wezterm_font_value_assignment_before_offset_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
        .map(str::to_owned)
        .or_else(|| {
            lua_static_wezterm_font_alias_query_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
        .or_else(|| {
            lua_static_wezterm_font_call_query_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
    });
    let value = resolved_value.as_deref().unwrap_or(value);
    let normalized_value = static_source.or(value_source).and_then(|static_source| {
        lua_static_wezterm_font_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
        .or_else(|| {
            lua_static_wezterm_font_call_query_from_query(
                static_source.source,
                value,
                static_source.max_start,
            )
        })
    });
    let value = normalized_value.as_deref().unwrap_or(value);
    Some(NativeFontConfig {
        families: parse_wezterm_font_families_value(source, value)?,
        attributes: parse_wezterm_font_attributes_value(source, value).unwrap_or_default(),
    })
}

fn parse_wezterm_font_attributes_value(source: &str, value: &str) -> Option<NativeFontAttributes> {
    let value = value.trim();
    if let Some(attributes) = parse_wezterm_font_with_fallback_attributes_value(source, value) {
        return Some(attributes);
    }
    if let Some(table) = wezterm_font_table_literal_from_query(value) {
        return native_font_attributes_lua_table_from_query(source, table);
    }
    let rest = lua_function_name_rest_from_query(value, "wezterm.font")?;
    let rest = rest.strip_prefix('(')?.trim_start();
    let literal = lua_quoted_string_literal_from_query(rest)?;
    let rest = lua_trim_start_comments(rest.get(literal.len()..)?)?;
    let rest = lua_trim_start_comments(rest.strip_prefix(',')?)?;
    let table = lua_braced_table_literal_from_query(rest)?;
    native_font_attributes_lua_table_from_query(source, table)
}

fn parse_wezterm_font_with_fallback_attributes_value(
    source: &str,
    value: &str,
) -> Option<NativeFontAttributes> {
    let value = value.trim();
    let mut rest = lua_function_name_rest_from_query(value, "wezterm.font_with_fallback")?;
    let parenthesized = rest.starts_with('(');
    if let Some(stripped) = rest.strip_prefix('(') {
        rest = stripped.trim_start();
    }
    let families = lua_braced_table_literal_from_query(rest)?;
    if parenthesized
        && let Some(rest) = lua_trim_start_comments(rest.get(families.len()..)?)
        && let Some(rest) = rest.strip_prefix(',')
        && let Some(rest) = lua_trim_start_comments(rest)
        && let Some(attributes) = lua_braced_table_literal_from_query(rest)
        && let Some(attributes) = native_font_attributes_lua_table_from_query(source, attributes)
    {
        return Some(attributes);
    }
    parse_wezterm_font_with_fallback_primary_attributes_value(source, value)
}

fn parse_wezterm_font_with_fallback_primary_attributes_value(
    source: &str,
    value: &str,
) -> Option<NativeFontAttributes> {
    let mut rest = lua_function_name_rest_from_query(value, "wezterm.font_with_fallback")?;
    if let Some(stripped) = rest.strip_prefix('(') {
        rest = stripped.trim_start();
    }
    let table = lua_braced_table_literal_from_query(rest)?;
    let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let field = split_lua_table_top_level_fields(table)?
        .into_iter()
        .find(|field| !field.trim().is_empty())?;
    let value = split_lua_table_assignment_from_field(field)
        .map_or(field, |(_, value)| value)
        .trim();
    let table = lua_braced_table_literal_from_query(value)?;
    lua_table_field_value_from_query(table, "family")??;
    native_font_attributes_lua_table_from_query(source, table)
}

fn wezterm_font_table_literal_from_query(value: &str) -> Option<&str> {
    let value = value.trim();
    let mut rest = lua_function_name_rest_from_query(value, "wezterm.font")?;
    if let Some(stripped) = rest.strip_prefix('(') {
        rest = stripped.trim_start();
    }
    lua_braced_table_literal_from_query(rest)
}

fn native_font_attributes_lua_table_from_query(
    source: &str,
    value: &str,
) -> Option<NativeFontAttributes> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut attributes = NativeFontAttributes::default();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query(key.trim())?;
        let value = lua_trim_start_comments(value)?;
        match key.as_str() {
            "weight" => {
                attributes.weight = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "stretch" => {
                attributes.stretch = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "style" => {
                attributes.style = Some(
                    lua_static_string_assignment_value_from_query(source, value)
                        .and_then(parse_maybe_quoted_query_text)?,
                );
            }
            "bold" => {
                if lua_static_bool_assignment_value_from_query(source, value)?
                    .parse()
                    .ok()?
                {
                    attributes.weight = Some("Bold".to_owned());
                }
            }
            "italic" => {
                if lua_static_bool_assignment_value_from_query(source, value)?
                    .parse()
                    .ok()?
                {
                    attributes.style = Some("Italic".to_owned());
                }
            }
            "harfbuzz_features" => {
                attributes.harfbuzz_features = split_lua_table_string_array(value)?;
            }
            "assume_emoji_presentation" => {
                attributes.assume_emoji_presentation = Some(
                    lua_static_bool_assignment_value_from_query(source, value)?
                        .parse()
                        .ok()?,
                );
            }
            "freetype_load_target" => {
                let value = lua_static_string_assignment_value_from_query(source, value)
                    .and_then(parse_maybe_quoted_query_text)?;
                attributes.freetype_load_target = Some(NativeFreetypeTarget::parse(&value)?);
            }
            "freetype_render_target" => {
                let value = lua_static_string_assignment_value_from_query(source, value)
                    .and_then(parse_maybe_quoted_query_text)?;
                attributes.freetype_render_target = Some(NativeFreetypeTarget::parse(&value)?);
            }
            "freetype_load_flags" => {
                let value = lua_static_string_assignment_value_from_query(source, value)
                    .and_then(parse_maybe_quoted_query_text)?;
                attributes.freetype_load_flags = Some(NativeFreetypeLoadFlags::parse(&value)?);
            }
            _ => {}
        }
    }

    Some(attributes)
}

fn parse_wezterm_font_with_fallback_families_value(
    source: &str,
    value: &str,
) -> Option<Vec<String>> {
    let value = value.trim();
    let mut rest = lua_function_name_rest_from_query(value, "wezterm.font_with_fallback")?;
    if let Some(stripped) = rest.strip_prefix('(') {
        rest = stripped.trim_start();
    }
    let table = lua_braced_table_literal_from_query(rest)?;
    let table = table.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut families = Vec::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some(family) = lua_static_string_assignment_value_from_query(source, field)
            .and_then(parse_maybe_quoted_query_text)
        {
            families.push(family);
            continue;
        }
        let Some(table) = lua_braced_table_literal_from_query(field) else {
            continue;
        };
        let Some(family) = lua_table_field_value_from_query(table, "family")?.and_then(|value| {
            lua_static_string_assignment_value_from_query(source, value)
                .and_then(parse_maybe_quoted_query_text)
        }) else {
            continue;
        };
        families.push(family);
    }

    (!families.is_empty()).then_some(families)
}

fn native_font_rules_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
    value: &str,
) -> Option<Vec<NativeFontRule>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut rules = Vec::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        rules.push(native_font_rule_lua_table_from_query(
            static_source,
            source,
            field,
        )?);
    }

    (!rules.is_empty()).then_some(rules)
}

fn parse_lua_static_bool_value_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
    value: &str,
) -> Option<bool> {
    if let Some(value) = lua_static_bool_assignment_value_from_query(source, value) {
        return value.parse().ok();
    }

    let static_source = static_source?;
    lua_static_bool_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
    )?
    .parse()
    .ok()
}

fn parse_lua_static_string_value_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
    value: &str,
) -> Option<String> {
    if let Some(value) = lua_static_string_assignment_value_from_query(source, value)
        .and_then(parse_maybe_quoted_query_text)
    {
        return Some(value);
    }

    let static_source = static_source?;
    lua_static_string_assignment_value_before_offset_from_query(
        static_source.source,
        value,
        static_source.max_start,
    )
    .and_then(parse_maybe_quoted_query_text)
}

fn native_font_rule_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    source: &str,
    value: &str,
) -> Option<NativeFontRule> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut rule = NativeFontRule::default();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let Some((key, value)) = split_lua_table_assignment_from_field(field) else {
            continue;
        };
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        let value = lua_trim_start_comments(value)?;
        match key.as_str() {
            "italic" => {
                rule.italic = Some(parse_lua_static_bool_value_with_static_source(
                    static_source,
                    source,
                    value,
                )?);
            }
            "intensity" => {
                let value =
                    parse_lua_static_string_value_with_static_source(static_source, source, value)?;
                rule.intensity = Some(tab_bar_item_intensity_from_query(&value)?);
            }
            "underline" => {
                let value =
                    parse_lua_static_string_value_with_static_source(static_source, source, value)?;
                rule.underline = Some(native_format_underline_from_query(&value)?);
            }
            "blink" => {
                let value =
                    parse_lua_static_string_value_with_static_source(static_source, source, value)?;
                rule.blink = Some(NativeFontRuleBlink::parse(&value)?);
            }
            "reverse" => {
                rule.reverse = Some(parse_lua_static_bool_value_with_static_source(
                    static_source,
                    source,
                    value,
                )?);
            }
            "strikethrough" => {
                rule.strikethrough = Some(parse_lua_static_bool_value_with_static_source(
                    static_source,
                    source,
                    value,
                )?);
            }
            "invisible" => {
                rule.invisible = Some(parse_lua_static_bool_value_with_static_source(
                    static_source,
                    source,
                    value,
                )?);
            }
            "font" => {
                let font_config = parse_wezterm_font_config_value_with_static_source(
                    static_source,
                    source,
                    value,
                )?;
                let mut families = font_config.families.into_iter();
                rule.font = families.next();
                rule.font_fallbacks = families.collect();
                rule.font_attributes = font_config.attributes;
            }
            _ => {}
        }
    }

    rule.font.as_ref()?;
    Some(rule)
}

#[allow(dead_code)]
fn native_hsb_multiplier_from_ratio(ratio: f32) -> Option<NativeHsbMultiplier> {
    native_non_negative_ratio_to_per_mille(ratio).map(NativeHsbMultiplier::from_per_mille)
}

#[allow(dead_code)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "opacity is clamped to 0..=1 before rounded per-mille conversion"
)]
fn native_text_background_opacity_from_alpha(alpha: f32) -> Option<NativeTextBackgroundOpacity> {
    if !alpha.is_finite() || alpha < 0.0 {
        return None;
    }
    let per_mille = (alpha.min(1.0) * 1_000.0).round();
    Some(NativeTextBackgroundOpacity::from_per_mille(
        per_mille as u16,
    ))
}

#[allow(dead_code)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "finite positive ratios are rounded and bounded to u16 before conversion"
)]
fn native_ratio_to_per_mille(ratio: f32) -> Option<u16> {
    if !ratio.is_finite() || ratio <= 0.0 {
        return None;
    }
    let per_mille = (ratio * 1_000.0).round();
    (per_mille <= f32::from(u16::MAX)).then_some(per_mille as u16)
}

#[allow(dead_code)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "finite nonnegative ratios are rounded and bounded to u16 before conversion"
)]
fn native_non_negative_ratio_to_per_mille(ratio: f32) -> Option<u16> {
    if !ratio.is_finite() || ratio < 0.0 {
        return None;
    }
    let per_mille = (ratio * 1_000.0).round();
    (per_mille <= f32::from(u16::MAX)).then_some(per_mille as u16)
}

#[allow(dead_code)]
fn native_easing_lua_value_from_query(value: &str) -> Option<NativeEasingFunction> {
    let value = value.trim();
    if value.starts_with('{') {
        return native_easing_lua_table_from_query(value);
    }

    if let Some(value) = parse_maybe_quoted_query_text(value) {
        return NativeEasingFunction::parse(&value);
    }

    None
}

#[allow(dead_code)]
fn native_easing_lua_table_from_query(value: &str) -> Option<NativeEasingFunction> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut fields = split_lua_table_top_level_fields(table)?
        .into_iter()
        .map(str::trim)
        .filter(|field| !field.is_empty());
    let field = fields.next()?;
    if fields.next().is_some() {
        return None;
    }
    let (key, value) = split_lua_table_assignment_from_field(field)?;
    let key = split_lua_table_key_from_query(key.trim())?;
    if key != "CubicBezier" {
        return None;
    }

    let value = lua_braced_table_literal_from_query(value.trim())?;
    let points = split_lua_table_f64_array(value)?;
    let [x1, y1, x2, y2] = points.as_slice() else {
        return None;
    };

    Some(NativeEasingFunction::CubicBezier(NativeCubicBezier {
        x1_per_mille: lua_easing_coordinate_per_mille(*x1),
        y1_per_mille: lua_easing_coordinate_per_mille(*y1),
        x2_per_mille: lua_easing_coordinate_per_mille(*x2),
        y2_per_mille: lua_easing_coordinate_per_mille(*y2),
    }))
}

#[allow(dead_code)]
fn lua_easing_coordinate_per_mille(value: f64) -> i32 {
    rounded_i32(value * 1_000.0)
}

#[allow(dead_code)]
fn lua_quoted_string_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    let quote = query
        .chars()
        .next()
        .filter(|quote| *quote == '\'' || *quote == '"')?;
    let mut escape = false;
    for (index, character) in query[quote.len_utf8()..].char_indices() {
        if escape {
            escape = false;
        } else if character == '\\' {
            escape = true;
        } else if character == quote {
            let end = quote.len_utf8() + index + character.len_utf8();
            return query.get(..end);
        }
    }
    None
}

#[allow(dead_code)]
fn lua_long_bracket_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    let (content_start, closing) = parse_lua_long_bracket_delimiters(query)?;
    let content_and_rest = &query[content_start..];
    let close_index = content_and_rest.find(&closing)?;
    query.get(..content_start + close_index + closing.len())
}

fn lua_bracket_starts_complete_long_string_index(query: &str) -> bool {
    let Some(after_outer_open) = query.strip_prefix('[') else {
        return false;
    };
    let inner = after_outer_open.trim_start();
    let Some(inner_literal) = lua_long_bracket_literal_from_query(inner) else {
        return false;
    };
    inner
        .get(inner_literal.len()..)
        .is_some_and(|rest| rest.trim_start().starts_with(']'))
}

#[allow(dead_code)]
fn lua_config_assignment_field_has_boundaries(source: &str, start: usize, field: &str) -> bool {
    let before = source[..start].chars().next_back();
    let after = source[start + field.len()..].chars().next();
    !before.is_some_and(is_lua_identifier_character)
        && !after.is_some_and(is_lua_identifier_character)
}

#[allow(dead_code)]
fn is_lua_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

#[allow(dead_code)]
fn lua_braced_table_literal_from_query(query: &str) -> Option<&str> {
    let query = query.trim_start();
    if !query.starts_with('{') {
        return None;
    }

    let mut depth = 0u32;
    let mut quote = None;
    let mut escape = false;
    let mut line_comment = false;
    let mut block_comment_end = None;
    let mut long_bracket_end = None;
    for (index, character) in query.char_indices() {
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

        if query[index..].starts_with("--") {
            if let Some((content_start, closing)) =
                parse_lua_long_bracket_delimiters(&query[index + 2..])
            {
                let content_and_rest = &query[index + 2 + content_start..];
                block_comment_end = Some(
                    content_and_rest
                        .find(&closing)
                        .map_or(query.len(), |close_index| {
                            index + 2 + content_start + close_index + closing.len()
                        }),
                );
                continue;
            }
            line_comment = true;
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            '[' => {
                if let Some((content_start, closing)) =
                    parse_lua_long_bracket_delimiters(&query[index..])
                {
                    let content_and_rest = &query[index + content_start..];
                    long_bracket_end = Some(
                        content_and_rest
                            .find(&closing)
                            .map_or(query.len(), |close_index| {
                                index + content_start + close_index + closing.len()
                            }),
                    );
                }
            }
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return query.get(..index + character.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}

fn native_leader_lua_table_from_query<'a>(
    source: &'a str,
    value: &'a str,
    max_start: usize,
) -> Option<NativeLeaderKey> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut key = None;
    let mut mods = None;
    let mut timeout_milliseconds = None;
    let static_source = Some(LuaStaticSource { source, max_start });

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("key") {
            if key.is_some() {
                return None;
            }
            key = Some(parse_maybe_static_query_text(static_source, value)?);
        } else if name.eq_ignore_ascii_case("mods") {
            if mods.is_some() {
                return None;
            }
            mods = Some(parse_maybe_static_query_text(static_source, value)?);
        } else if name.eq_ignore_ascii_case("timeout_milliseconds")
            || name.eq_ignore_ascii_case("timeout-milliseconds")
        {
            if timeout_milliseconds.is_some() {
                return None;
            }
            timeout_milliseconds = Some(
                lua_static_number_assignment_value_before_offset_from_query(
                    source,
                    value,
                    max_start,
                    lua_unsigned_integer_literal_from_query,
                )?
                .parse()
                .ok()?,
            );
        } else {
            return None;
        }
    }

    let key = non_empty_spawn_command_option_value(&key?).ok()?;
    let mods = match mods {
        Some(mods) => Some(non_empty_spawn_command_option_value(&mods).ok()?),
        None => None,
    };
    let keys = mods.map_or(key.clone(), |mods| format!("{mods}+{key}"));
    Some(NativeLeaderKey {
        keys,
        timeout_milliseconds,
    })
}

fn native_launch_menu_lua_table_from_query(
    source: &str,
    value: &str,
    max_start: usize,
) -> Option<Vec<NativeLaunchMenuItem>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut items = Vec::new();
    let mut indexed_items = BTreeMap::new();
    let static_source = Some(LuaStaticSource { source, max_start });

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !items.is_empty() || index == 0 || indexed_items.contains_key(&index) {
                return None;
            }
            indexed_items.insert(
                index,
                native_launch_menu_item_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_items.is_empty() {
            return None;
        }
        items.push(native_launch_menu_item_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_items.is_empty() {
        return (1..=indexed_items.len())
            .map(|index| indexed_items.remove(&index))
            .collect();
    }

    Some(items)
}

#[derive(Clone, Copy)]
struct LuaStaticSource<'a> {
    source: &'a str,
    max_start: usize,
}

fn native_launch_menu_item_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeLaunchMenuItem> {
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
    let mut label = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field)?;
        let key = split_lua_table_key_from_query_with_static_source(static_source, key.trim())?;
        if key.eq_ignore_ascii_case("label") {
            if label.is_some() {
                return None;
            }
            let value = parse_maybe_static_query_text(static_source, value.trim())?;
            label = Some(non_empty_spawn_command_option_value(&value).ok()?);
        }
    }

    Some(NativeLaunchMenuItem {
        label,
        command: native_launch_menu_command_lua_table_from_query(static_source, value)?,
    })
}

fn native_launch_menu_command_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeLaunchMenuCommand> {
    spawn_command_table_from_query_with_static_source(static_source, value, false)
        .map(NativeLaunchMenuCommand::Command)
        .or_else(|| {
            spawn_command_table_options_from_query_with_static_source(static_source, value, false)
                .map(NativeLaunchMenuCommand::Options)
        })
}

fn native_key_tables_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<BTreeMap<String, Vec<NativeUserKeyAssignment>>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut key_tables = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        if key_tables.contains_key(&name) {
            return None;
        }
        key_tables.insert(
            name,
            native_key_assignments_lua_table_from_query(static_source, value.trim())?,
        );
    }

    Some(key_tables)
}

fn native_key_assignments_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<NativeUserKeyAssignment>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut assignments = Vec::new();
    let mut indexed_assignments = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !assignments.is_empty() || index == 0 || indexed_assignments.contains_key(&index) {
                return None;
            }
            indexed_assignments.insert(
                index,
                native_user_key_assignment_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_assignments.is_empty() {
            return None;
        }
        assignments.push(native_user_key_assignment_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_assignments.is_empty() {
        return (1..=indexed_assignments.len())
            .map(|index| indexed_assignments.remove(&index))
            .collect();
    }

    Some(assignments)
}

fn native_user_key_assignment_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeUserKeyAssignment> {
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
    let mut key = None;
    let mut mods = None;
    let mut command = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "key" => {
                if key.is_some() {
                    return None;
                }
                key = Some(parse_maybe_static_query_text(static_source, value)?);
            }
            "mods" | "mod" => {
                if mods.is_some() {
                    return None;
                }
                mods = Some(parse_maybe_static_query_text(static_source, value)?);
            }
            "action" => {
                if command.is_some() {
                    return None;
                }
                command = Some(native_key_assignment_command_from_query(
                    static_source,
                    value,
                )?);
            }
            _ => return None,
        }
    }

    let key = key.filter(|key| !key.is_empty())?;
    let keys = match mods {
        Some(mods) if !mods.eq_ignore_ascii_case("NONE") => format!("{mods}+{key}"),
        _ => key,
    };

    Some(NativeUserKeyAssignment {
        keys,
        command: command?,
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
fn native_key_assignment_command_from_query(
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
            return native_key_assignment_command_from_query(Some(static_source), &value);
        }
        return native_key_assignment_command_from_query(None, value);
    }
    if let Some(static_source) = static_source
        && let Some(value) = lua_static_wezterm_action_alias_query_from_query(
            static_source.source,
            value,
            static_source.max_start,
        )
    {
        return native_key_assignment_command_from_query(Some(static_source), &value);
    }
    if let Some(static_source) = static_source
        && let Some(commands) =
            multiple_table_commands_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::Multiple(commands));
    }
    if let Some(static_source) = static_source
        && let Some(options) =
            quick_select_lua_table_from_query_with_static_source(Some(static_source), value)
    {
        return Some(WindowCommand::QuickSelectArgs(options));
    }
    if let Some(options) = pane_select_options_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::PaneSelect(options));
    }
    if let Some(options) = char_select_options_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::CharSelectArgs(options));
    }
    if let Some(args) = show_launcher_args_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::ShowLauncherArgs(args));
    }
    if let Some(options) =
        prompt_input_line_options_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::PromptInputLine(options));
    }
    if let Some(options) =
        input_selector_options_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::InputSelector(options));
    }
    if let Some(options) = confirmation_options_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::Confirmation(options));
    }
    if let Some(event) = emit_event_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::EmitEvent(event));
    }
    if let Some(uri) = open_uri_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::OpenUri(uri));
    }
    if let Some((text, destination)) =
        copy_text_to_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::CopyTextTo { text, destination });
    }
    if let Some(value) = send_string_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::SendString(value));
    }
    if let Some(send_key) = send_key_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::SendKey(send_key));
    }
    if let Some(key_table) = activate_key_table_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::ActivateKeyTable(key_table));
    }
    if let Some(spawn_command) =
        spawn_command_in_new_tab_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SpawnCommandInNewTab(spawn_command));
    }
    if let Some(spawn_options) =
        spawn_command_options_in_new_tab_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SpawnCommandOptionsInNewTab(spawn_options));
    }
    if let Some(spawn_command) =
        spawn_command_in_new_window_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SpawnCommandInNewWindow(spawn_command));
    }
    if let Some(spawn_options) =
        spawn_command_options_in_new_window_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SpawnCommandOptionsInNewWindow(spawn_options));
    }
    if let Some(domain) = spawn_tab_domain_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::SpawnTab(domain));
    }
    if let Some(domain) = attach_domain_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::AttachDomain(domain));
    }
    if let Some(domain) = detach_domain_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::DetachDomain(domain));
    }
    if let Some(split_pane) =
        split_pane_table_action_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SplitPane(split_pane));
    }
    if let Some(command) = close_current_command_from_query_with_static_source(static_source, value)
    {
        return Some(command);
    }
    if let Some(command) =
        activate_window_command_from_query_with_static_source(static_source, value)
    {
        return Some(command);
    }
    if let Some(assignment) =
        copy_mode_assignment_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::CopyMode(assignment));
    }
    if let Some(index) = activate_tab_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::ActivateTab(index));
    }
    if let Some(offset) =
        activate_tab_relative_no_wrap_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::ActivateTabRelativeNoWrap(offset));
    }
    if let Some(offset) = activate_tab_relative_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::ActivateTabRelative(offset));
    }
    if let Some(index) = move_tab_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::MoveTab(index));
    }
    if let Some(window_id) = move_tab_to_window_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::MoveTabToWindow(window_id));
    }
    if let Some(offset) = move_tab_relative_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::MoveTabRelative(offset));
    }
    if let Some(index) = activate_pane_by_index_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::ActivatePaneByIndex(index));
    }
    if let Some(direction) =
        activate_pane_direction_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::ActivatePaneDirection(direction));
    }
    if let Some(direction) = rotate_panes_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::RotatePanes(direction));
    }
    if let Some((direction, amount)) =
        adjust_pane_size_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::AdjustPaneSize { direction, amount });
    }
    if let Some(amount) = scroll_by_page_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::ScrollByPage(amount));
    }
    if let Some(amount) = scroll_by_line_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::ScrollByLine(amount));
    }
    if let Some(amount) = scroll_to_prompt_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::ScrollToPrompt(amount));
    }
    if let Some(zoomed) = set_pane_zoom_state_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::SetPaneZoomState(zoomed));
    }
    if let Some(mode) = clear_scrollback_mode_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::ClearScrollback(mode));
    }
    if let Some(destination) =
        copy_destination_command_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::CopyTo(destination));
    }
    if let Some(source) = paste_source_command_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::PasteFrom(source));
    }
    if let Some(destination) =
        complete_selection_destination_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::CompleteSelectionTo(destination));
    }
    if let Some(destination) =
        complete_selection_or_open_link_destination_from_query_with_static_source(
            static_source,
            value,
        )
    {
        return Some(WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursorTo(
            destination,
        ));
    }
    if let Some(mode) =
        select_text_at_mouse_cursor_mode_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SelectTextAtMouseCursor(mode));
    }
    if let Some(mode) =
        extend_selection_to_mouse_cursor_mode_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::ExtendSelectionToMouseCursor(mode));
    }
    if let Some(level) = set_window_level_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::SetWindowLevel(level));
    }
    if let Some(offset) =
        switch_workspace_relative_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SwitchWorkspaceRelative(offset));
    }
    if let Some(title) = rename_tab_title_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::RenameTabTo(title));
    }
    if let Some(name) = rename_workspace_name_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::RenameWorkspaceTo(name));
    }
    if let Some(search_query) = search_query_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::Search(search_query));
    }
    if let Some(options) =
        switch_workspace_options_from_query_with_static_source(static_source, value)
    {
        return Some(WindowCommand::SwitchToWorkspaceArgs(options));
    }
    if let Some(command) =
        wezterm_action_table_wrapper_command_with_static_source(static_source, value)
    {
        return Some(command);
    }
    if let Some(command) = key_table_stack_command_from_query(value) {
        return Some(command);
    }
    if let Some(command) =
        lua_action_callback_perform_action_command_with_static_source(static_source, value)
    {
        return Some(command);
    }
    if lua_action_callback_from_query_with_static_source(static_source, value) {
        return Some(WindowCommand::Nop);
    }
    command_palette_structured_query_command(value)
}

fn native_mouse_assignments_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<Vec<NativeUserMouseAssignment>> {
    let table = value.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let mut assignments = Vec::new();
    let mut indexed_assignments = BTreeMap::new();

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_lua_table_assignment_from_field(field)
            && let Some(index) = split_lua_table_array_index_from_query(key.trim())
        {
            if !assignments.is_empty() || index == 0 || indexed_assignments.contains_key(&index) {
                return None;
            }
            indexed_assignments.insert(
                index,
                native_user_mouse_assignment_lua_table_from_query(static_source, value.trim())?,
            );
            continue;
        }

        if !indexed_assignments.is_empty() {
            return None;
        }
        assignments.push(native_user_mouse_assignment_lua_table_from_query(
            static_source,
            field,
        )?);
    }

    if !indexed_assignments.is_empty() {
        return (1..=indexed_assignments.len())
            .map(|index| indexed_assignments.remove(&index))
            .collect();
    }

    Some(assignments)
}

fn native_user_mouse_assignment_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeUserMouseAssignment> {
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
    let mut event = None;
    let mut modifiers = None;
    let mut mouse_reporting = None;
    let mut alt_screen = None;
    let mut command = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "event" => {
                if event.is_some() {
                    return None;
                }
                event = Some(native_mouse_assignment_event_lua_table_from_query(
                    static_source,
                    value,
                )?);
            }
            "mods" | "mod" => {
                if modifiers.is_some() {
                    return None;
                }
                let value = parse_maybe_static_query_text(static_source, value)?;
                modifiers = Some(native_modifiers_from_wezterm_lua_config(&value)?);
            }
            "action" => {
                if command.is_some() {
                    return None;
                }
                command = Some(native_key_assignment_command_from_query(
                    static_source,
                    value,
                )?);
            }
            "mouse_reporting" | "mousereporting" => {
                if mouse_reporting.is_some() {
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
                mouse_reporting = Some(bool_from_query(&value)?);
            }
            "alt_screen" | "altscreen" => {
                if alt_screen.is_some() {
                    return None;
                }
                let value = if let Some(static_source) = static_source {
                    lua_static_bool_assignment_value_before_offset_from_query(
                        static_source.source,
                        value,
                        static_source.max_start,
                    )
                    .map(str::to_owned)
                    .or_else(|| parse_maybe_static_query_text(Some(static_source), value))?
                } else {
                    parse_maybe_quoted_query_text(value)?
                };
                alt_screen = Some(native_mouse_assignment_alt_screen_from_query(&value)?);
            }
            _ => return None,
        }
    }

    Some(NativeUserMouseAssignment {
        event: event?,
        modifiers: modifiers.unwrap_or_else(ModifiersState::empty),
        mouse_reporting: mouse_reporting.unwrap_or(false),
        alt_screen: alt_screen.unwrap_or(NativeMouseAssignmentAltScreen::Any),
        command: command?,
    })
}

fn native_mouse_assignment_alt_screen_from_query(
    value: &str,
) -> Option<NativeMouseAssignmentAltScreen> {
    if value.eq_ignore_ascii_case("any") {
        return Some(NativeMouseAssignmentAltScreen::Any);
    }
    bool_from_query(value).map(NativeMouseAssignmentAltScreen::Active)
}

fn native_mouse_assignment_event_lua_table_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeMouseAssignmentEvent> {
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
    let mut parsed = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        if parsed.is_some() {
            return None;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let kind = match name.to_ascii_lowercase().as_str() {
            "down" => NativeMouseAssignmentEventKind::Down,
            "up" => NativeMouseAssignmentEventKind::Up,
            "drag" => NativeMouseAssignmentEventKind::Drag,
            _ => return None,
        };
        parsed = Some(native_mouse_assignment_event_payload_from_query(
            static_source,
            value.trim(),
            kind,
        )?);
    }

    parsed
}

fn native_mouse_assignment_event_payload_from_query(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
    kind: NativeMouseAssignmentEventKind,
) -> Option<NativeMouseAssignmentEvent> {
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
    let mut button = None;
    let mut streak = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "button" => {
                if button.is_some() {
                    return None;
                }
                button = Some(native_mouse_assignment_button_from_lua_value(
                    static_source,
                    value,
                )?);
            }
            "streak" => {
                if streak.is_some() {
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
                streak = Some(value.parse::<u8>().ok()?);
            }
            _ => return None,
        }
    }

    let streak = streak?;
    if streak == 0 {
        return None;
    }

    Some(NativeMouseAssignmentEvent {
        kind,
        button: button?,
        streak,
    })
}

fn native_mouse_assignment_button_from_lua_value(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<NativeMouseAssignmentButton> {
    let value = value.trim();
    let resolved_value;
    let maybe_table = if value.starts_with('{') {
        Some(value)
    } else if let Some(static_source) = static_source {
        resolved_value = lua_table_insert_value_table_string_from_query(
            static_source.source,
            value,
            static_source.max_start,
        );
        resolved_value.as_deref()
    } else {
        None
    };
    if let Some(value) = maybe_table {
        let table = value.strip_prefix('{')?.strip_suffix('}')?.trim();
        let mut parsed = None;
        for field in split_lua_table_top_level_fields(table)? {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            if parsed.is_some() {
                return None;
            }
            let (name, value) = split_lua_table_assignment_from_field(field)?;
            let name =
                split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
            let value = value.trim();
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
            };
            if amount.parse::<i32>().ok()? != 1 {
                return None;
            }
            parsed = match name.to_ascii_lowercase().as_str() {
                "wheelup" => Some(NativeMouseAssignmentButton::WheelUp),
                "wheeldown" => Some(NativeMouseAssignmentButton::WheelDown),
                _ => return None,
            };
        }
        return parsed;
    }

    let value = parse_maybe_static_query_text(static_source, value)?;
    match value.trim().to_ascii_lowercase().as_str() {
        "left" => Some(NativeMouseAssignmentButton::Mouse(MouseButton::Left)),
        "middle" => Some(NativeMouseAssignmentButton::Mouse(MouseButton::Middle)),
        "right" => Some(NativeMouseAssignmentButton::Mouse(MouseButton::Right)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeKeyMapPreference {
    #[default]
    Mapped,
    Physical,
}

impl NativeKeyMapPreference {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "Mapped" => Some(Self::Mapped),
            "Physical" => Some(Self::Physical),
            _ => None,
        }
    }

    fn config_text(self) -> &'static str {
        match self {
            Self::Mapped => "Mapped",
            Self::Physical => "Physical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeUiKeyCapRendering {
    UnixLong,
    Emacs,
    AppleSymbols,
    WindowsLong,
    WindowsSymbols,
}

impl NativeUiKeyCapRendering {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "UnixLong" => Some(Self::UnixLong),
            "Emacs" => Some(Self::Emacs),
            "AppleSymbols" => Some(Self::AppleSymbols),
            "WindowsLong" => Some(Self::WindowsLong),
            "WindowsSymbols" => Some(Self::WindowsSymbols),
            _ => None,
        }
    }

    fn config_text(self) -> &'static str {
        match self {
            Self::UnixLong => "UnixLong",
            Self::Emacs => "Emacs",
            Self::AppleSymbols => "AppleSymbols",
            Self::WindowsLong => "WindowsLong",
            Self::WindowsSymbols => "WindowsSymbols",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeTabTitle {
    Text(String),
    Format(Vec<NativeFormatItem>),
}

impl NativeTabTitle {
    fn plain_text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Format(items) => items
                .iter()
                .filter_map(|item| match item {
                    NativeFormatItem::Text(text) => Some(tab_bar_ansi_plain_text(text)),
                    NativeFormatItem::Foreground(_)
                    | NativeFormatItem::Background(_)
                    | NativeFormatItem::Attribute(_)
                    | NativeFormatItem::ResetAttributes => None,
                })
                .collect(),
        }
    }
}

impl From<String> for NativeTabTitle {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaTabTitle {
    Static(NativeTabTitle),
    TabId,
    TabIndex {
        offset: usize,
    },
    TabCount,
    PaneCount,
    ActiveTabTitle,
    WindowTitle,
    Conditional {
        branches: Vec<NativeLuaTabTitleConditionalBranch>,
        fallback: Box<NativeLuaTabTitle>,
    },
    Concat(Vec<NativeLuaTabTitleTextPart>),
    Format(Vec<NativeLuaFormatItem>),
    ActivePaneDomainName,
    ActivePaneForegroundProcessName,
    ActivePaneCurrentWorkingDir,
    ActivePaneTtyName,
    ActivePaneTitle,
}

impl NativeLuaTabTitle {
    fn resolve(&self, event: &NativeTabTitleFormat) -> Option<NativeTabTitle> {
        match self {
            Self::Static(title) => Some(title.clone()),
            Self::TabId => Some(NativeTabTitle::Text(event.tab.get().to_string())),
            Self::TabIndex { offset } => Some(NativeTabTitle::Text(
                event.tab_index.saturating_add(*offset).to_string(),
            )),
            Self::TabCount => Some(NativeTabTitle::Text(event.tab_count.to_string())),
            Self::PaneCount => Some(NativeTabTitle::Text(event.pane_count.to_string())),
            Self::ActiveTabTitle => event.tab_title.clone().map(NativeTabTitle::Text),
            Self::WindowTitle => Some(NativeTabTitle::Text(event.window_title.clone())),
            Self::Conditional { branches, fallback } => {
                for branch in branches {
                    if branch.condition.matches(event) {
                        return branch.title.resolve(event);
                    }
                }
                fallback.resolve(event)
            }
            Self::Concat(parts) => {
                let mut title = String::new();
                for part in parts {
                    title.push_str(&part.resolve(event)?);
                }
                Some(NativeTabTitle::Text(title))
            }
            Self::Format(items) => {
                let mut resolved = Vec::new();
                for item in items {
                    resolved.push(item.resolve(event)?);
                }
                Some(NativeTabTitle::Format(resolved))
            }
            Self::ActivePaneDomainName => Some(NativeTabTitle::Text(
                event.active_pane_info.domain_name.clone(),
            )),
            Self::ActivePaneForegroundProcessName => Some(NativeTabTitle::Text(
                event.active_pane_info.foreground_process_name.clone(),
            )),
            Self::ActivePaneCurrentWorkingDir => event
                .active_pane_info
                .current_working_dir
                .clone()
                .map(NativeTabTitle::Text),
            Self::ActivePaneTtyName => event
                .active_pane_info
                .tty_name
                .clone()
                .map(NativeTabTitle::Text),
            Self::ActivePaneTitle => event
                .active_pane_info
                .title
                .clone()
                .map(NativeTabTitle::Text),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaTabTitleConditionalBranch {
    condition: NativeLuaTabTitleCondition,
    title: NativeLuaTabTitle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaTabTitleCondition {
    IsActive,
    IsLastActive,
    IsHover,
    ActivePaneIsZoomed,
    ActivePaneProgressIndeterminate,
    ActivePaneProgressFieldPresence {
        field: NativeLuaTabTitleProgressField,
        present: bool,
    },
    ActivePaneUserVarPresence {
        name: String,
        present: bool,
    },
    TabCountGreaterThan(usize),
    PaneCountGreaterThan(usize),
}

impl NativeLuaTabTitleCondition {
    fn matches(&self, event: &NativeTabTitleFormat) -> bool {
        match self {
            Self::IsActive => event.is_active,
            Self::IsLastActive => event.is_last_active,
            Self::IsHover => event.hover,
            Self::ActivePaneIsZoomed => event.active_pane_info.is_zoomed,
            Self::ActivePaneProgressIndeterminate => {
                event.active_pane_info.progress == PaneProgress::Indeterminate
            }
            Self::ActivePaneProgressFieldPresence { field, present } => {
                matches!(
                    (*field, event.active_pane_info.progress),
                    (
                        NativeLuaTabTitleProgressField::Percentage,
                        PaneProgress::Percentage(_)
                    ) | (
                        NativeLuaTabTitleProgressField::Error,
                        PaneProgress::Error(_)
                    )
                ) == *present
            }
            Self::ActivePaneUserVarPresence { name, present } => {
                event.active_pane_info.user_vars.contains_key(name) == *present
            }
            Self::TabCountGreaterThan(count) => event.tab_count > *count,
            Self::PaneCountGreaterThan(count) => event.pane_count > *count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaFormatItem {
    Static(NativeFormatItem),
    Text(Vec<NativeLuaTabTitleTextPart>),
}

impl NativeLuaFormatItem {
    fn resolve(&self, event: &NativeTabTitleFormat) -> Option<NativeFormatItem> {
        match self {
            Self::Static(item) => Some(item.clone()),
            Self::Text(parts) => {
                let mut text = String::new();
                for part in parts {
                    text.push_str(&part.resolve(event)?);
                }
                Some(NativeFormatItem::Text(text))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaTabTitleTextPart {
    Static(String),
    TabId,
    TabIndex {
        offset: usize,
    },
    TabIndexAndCount {
        format: NativeLuaWindowTitleNumberPairFormat,
        tab_index_offset: usize,
    },
    TabCount,
    PaneCount,
    ActiveTabTitle,
    ActiveTabTitleOrActivePaneTitle,
    TruncateLeft {
        parts: Vec<NativeLuaTabTitleTextPart>,
        max_width_offset: usize,
    },
    TruncateRight {
        parts: Vec<NativeLuaTabTitleTextPart>,
        max_width_offset: usize,
    },
    WindowTitle,
    ActivePaneId,
    ActivePaneUserVar {
        name: String,
    },
    ActivePaneProgress {
        field: NativeLuaTabTitleProgressField,
    },
    ActivePaneDomainName,
    ActivePaneForegroundProcessName,
    ActivePaneCurrentWorkingDir,
    ActivePaneTtyName,
    ActivePaneTitle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaTabTitleProgressField {
    Percentage,
    Error,
}

impl NativeLuaTabTitleTextPart {
    fn resolve(&self, event: &NativeTabTitleFormat) -> Option<String> {
        match self {
            Self::Static(value) => Some(value.clone()),
            Self::TabId => Some(event.tab.get().to_string()),
            Self::TabIndex { offset } => Some(event.tab_index.saturating_add(*offset).to_string()),
            Self::TabIndexAndCount {
                format,
                tab_index_offset,
            } => Some(format.format(
                event.tab_index.saturating_add(*tab_index_offset),
                event.tab_count,
            )),
            Self::TabCount => Some(event.tab_count.to_string()),
            Self::PaneCount => Some(event.pane_count.to_string()),
            Self::ActiveTabTitle => event.tab_title.clone(),
            Self::ActiveTabTitleOrActivePaneTitle => event
                .tab_title
                .as_ref()
                .filter(|title| !title.is_empty())
                .cloned()
                .or_else(|| event.active_pane_info.title.clone()),
            Self::TruncateLeft {
                parts,
                max_width_offset,
            } => {
                let mut text = String::new();
                for part in parts {
                    text.push_str(&part.resolve(event)?);
                }
                let max_width = event.max_width.saturating_sub(*max_width_offset);
                Some(tab_bar_truncate_left(&text, max_width))
            }
            Self::TruncateRight {
                parts,
                max_width_offset,
            } => {
                let mut text = String::new();
                for part in parts {
                    text.push_str(&part.resolve(event)?);
                }
                let max_width = event.max_width.saturating_sub(*max_width_offset);
                Some(tab_bar_truncate_right(&text, max_width))
            }
            Self::WindowTitle => Some(event.window_title.clone()),
            Self::ActivePaneId => Some(event.active_pane_info.pane_id.get().to_string()),
            Self::ActivePaneUserVar { name } => event.active_pane_info.user_vars.get(name).cloned(),
            Self::ActivePaneProgress { field } => match (field, event.active_pane_info.progress) {
                (NativeLuaTabTitleProgressField::Percentage, PaneProgress::Percentage(value))
                | (NativeLuaTabTitleProgressField::Error, PaneProgress::Error(value)) => {
                    Some(value.to_string())
                }
                _ => None,
            },
            Self::ActivePaneDomainName => Some(event.active_pane_info.domain_name.clone()),
            Self::ActivePaneForegroundProcessName => {
                Some(event.active_pane_info.foreground_process_name.clone())
            }
            Self::ActivePaneCurrentWorkingDir => event.active_pane_info.current_working_dir.clone(),
            Self::ActivePaneTtyName => event.active_pane_info.tty_name.clone(),
            Self::ActivePaneTitle => event.active_pane_info.title.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeLuaTabTitleTruncateDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowTitle {
    Static(String),
    ActiveTabTitle,
    ActivePaneTitle,
    Concat(Vec<NativeLuaWindowTitlePart>),
}

impl NativeLuaWindowTitle {
    fn resolve(&self, event: &NativeWindowTitleFormat) -> Option<String> {
        match self {
            Self::Static(title) => Some(title.clone()),
            Self::ActiveTabTitle => event.active_tab_info.tab_title.clone(),
            Self::ActivePaneTitle => event.active_pane_info.title.clone(),
            Self::Concat(parts) => {
                let mut title = String::new();
                for part in parts {
                    title.push_str(&part.resolve(event)?);
                }
                Some(title)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowTitlePart {
    Static(String),
    ActiveTabId,
    ActiveTabIndex {
        offset: usize,
    },
    ActiveTabTitle,
    WindowTitle,
    ActivePaneId,
    ActivePaneUserVar {
        name: String,
    },
    ActivePaneProgress {
        field: NativeLuaTabTitleProgressField,
    },
    ActivePaneTitle,
    ActiveTabTitleOrActivePaneTitle,
    ActivePaneDomainName,
    ActivePaneForegroundProcessName,
    ActivePaneCurrentWorkingDir,
    ActivePaneTtyName,
    TabCount,
    PaneCount,
    Conditional {
        condition: NativeLuaWindowTitleCondition,
        parts: Vec<NativeLuaWindowTitlePart>,
        else_parts: Option<Vec<NativeLuaWindowTitlePart>>,
    },
    TabIndexAndCount {
        format: NativeLuaWindowTitleNumberPairFormat,
        tab_index_offset: usize,
    },
}

impl NativeLuaWindowTitlePart {
    fn resolve(&self, event: &NativeWindowTitleFormat) -> Option<String> {
        match self {
            Self::Static(value) => Some(value.clone()),
            Self::ActiveTabId => Some(event.active_tab_info.tab_id.get().to_string()),
            Self::ActiveTabIndex { offset } => {
                Some((event.active_tab_info.tab_index + offset).to_string())
            }
            Self::ActiveTabTitle => event.active_tab_info.tab_title.clone(),
            Self::WindowTitle => Some(event.active_tab_info.window_title.clone()),
            Self::ActivePaneId => Some(event.active_pane_info.pane_id.get().to_string()),
            Self::ActivePaneUserVar { name } => event.active_pane_info.user_vars.get(name).cloned(),
            Self::ActivePaneProgress { field } => match (field, event.active_pane_info.progress) {
                (NativeLuaTabTitleProgressField::Percentage, PaneProgress::Percentage(value))
                | (NativeLuaTabTitleProgressField::Error, PaneProgress::Error(value)) => {
                    Some(value.to_string())
                }
                _ => None,
            },
            Self::ActivePaneTitle => event.active_pane_info.title.clone(),
            Self::ActiveTabTitleOrActivePaneTitle => event
                .active_tab_info
                .tab_title
                .as_ref()
                .filter(|title| !title.is_empty())
                .cloned()
                .or_else(|| event.active_pane_info.title.clone()),
            Self::ActivePaneDomainName => Some(event.active_pane_info.domain_name.clone()),
            Self::ActivePaneForegroundProcessName => {
                Some(event.active_pane_info.foreground_process_name.clone())
            }
            Self::ActivePaneCurrentWorkingDir => event.active_pane_info.current_working_dir.clone(),
            Self::ActivePaneTtyName => event.active_pane_info.tty_name.clone(),
            Self::TabCount => Some(event.tab_count.to_string()),
            Self::PaneCount => Some(event.pane_count.to_string()),
            Self::Conditional {
                condition,
                parts,
                else_parts,
            } => {
                let parts = if condition.matches(event) {
                    parts
                } else if let Some(else_parts) = else_parts {
                    else_parts
                } else {
                    return Some(String::new());
                };
                let mut title = String::new();
                for part in parts {
                    title.push_str(&part.resolve(event)?);
                }
                Some(title)
            }
            Self::TabIndexAndCount {
                format,
                tab_index_offset,
            } => Some(
                format.format(
                    event
                        .active_tab_info
                        .tab_index
                        .saturating_add(*tab_index_offset),
                    event.tab_count,
                ),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeLuaWindowTitleCondition {
    ActivePaneIsZoomed,
    ActivePaneProgressIndeterminate,
    ActivePaneProgressFieldPresence {
        field: NativeLuaTabTitleProgressField,
        present: bool,
    },
    ActivePaneUserVarPresence {
        name: String,
        present: bool,
    },
    TabCountGreaterThan(usize),
    PaneCountGreaterThan(usize),
}

impl NativeLuaWindowTitleCondition {
    fn matches(&self, event: &NativeWindowTitleFormat) -> bool {
        match self {
            Self::ActivePaneIsZoomed => event.active_pane_info.is_zoomed,
            Self::ActivePaneProgressIndeterminate => {
                event.active_pane_info.progress == PaneProgress::Indeterminate
            }
            Self::ActivePaneProgressFieldPresence { field, present } => {
                let has_field = matches!(
                    (field, event.active_pane_info.progress),
                    (
                        &NativeLuaTabTitleProgressField::Percentage,
                        PaneProgress::Percentage(_)
                    ) | (
                        &NativeLuaTabTitleProgressField::Error,
                        PaneProgress::Error(_)
                    )
                );
                has_field == *present
            }
            Self::ActivePaneUserVarPresence { name, present } => {
                event.active_pane_info.user_vars.contains_key(name) == *present
            }
            Self::TabCountGreaterThan(count) => event.tab_count > *count,
            Self::PaneCountGreaterThan(count) => event.pane_count > *count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaWindowTitleNumberPairFormat {
    prefix: String,
    middle: String,
    suffix: String,
}

impl NativeLuaWindowTitleNumberPairFormat {
    fn parse(value: &str) -> Option<Self> {
        let first = value.find("%d")?;
        let after_first = first.checked_add("%d".len())?;
        let second = value.get(after_first..)?.find("%d")?;
        let second = after_first.checked_add(second)?;
        let after_second = second.checked_add("%d".len())?;
        if value.get(after_second..)?.contains("%d") {
            return None;
        }
        Some(Self {
            prefix: value.get(..first)?.to_owned(),
            middle: value.get(after_first..second)?.to_owned(),
            suffix: value.get(after_second..)?.to_owned(),
        })
    }

    fn format(&self, first: usize, second: usize) -> String {
        format!(
            "{}{}{}{}{}",
            self.prefix, first, self.middle, second, self.suffix
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFormatItem {
    Text(String),
    Foreground(Color),
    Background(Color),
    Attribute(NativeFormatAttribute),
    ResetAttributes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFormatAttribute {
    Intensity(NativeFormatIntensity),
    Italic(bool),
    Underline(NativeFormatUnderline),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFormatIntensity {
    Normal,
    Bold,
    Half,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NativeFormatUnderline {
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowStatusUpdateEvent {
    window_id: rterm_core::WindowId,
    pane: rterm_core::PaneId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowStatusUpdate {
    left_status: Option<String>,
    right_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLuaWindowStatusUpdate {
    left_status: Option<NativeLuaWindowStatusText>,
    right_status: Option<NativeLuaWindowStatusText>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeWindowConfigPatch(Box<NativeWindowConfigPatchValues>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeWindowConfigPatchValues {
    dpi: Option<u32>,
    dpi_by_screen: Option<BTreeMap<String, u32>>,
    font: Option<String>,
    font_fallbacks: Option<Vec<String>>,
    font_attributes: Option<NativeFontAttributes>,
    font_rules: Option<Vec<NativeFontRule>>,
    font_size: Option<NativeFontSize>,
    cell_width: Option<NativeCellWidth>,
    cell_widths: Option<Vec<NativeCellWidthOverride>>,
    line_height: Option<NativeLineHeight>,
    font_antialias: Option<NativeFontAntialias>,
    font_hinting: Option<NativeFontHinting>,
    font_rasterizer: Option<NativeFontRasterizer>,
    font_colr_rasterizer: Option<NativeFontRasterizer>,
    font_shaper: Option<NativeFontShaper>,
    harfbuzz_features: Option<Vec<String>>,
    font_dirs: Option<Vec<String>>,
    font_locator: Option<NativeFontLocator>,
    use_cap_height_to_scale_fallback_fonts: Option<bool>,
    ignore_svg_fonts: Option<bool>,
    sort_fallback_fonts_by_coverage: Option<bool>,
    search_font_dirs_for_fallback: Option<bool>,
    custom_block_glyphs: Option<bool>,
    anti_alias_custom_block_glyphs: Option<bool>,
    allow_square_glyphs_to_overflow_width: Option<NativeSquareGlyphOverflow>,
    freetype_load_target: Option<NativeFreetypeTarget>,
    freetype_render_target: Option<NativeFreetypeTarget>,
    freetype_load_flags: Option<NativeFreetypeLoadFlags>,
    freetype_interpreter_version: Option<u32>,
    freetype_pcf_long_family_names: Option<bool>,
    display_pixel_geometry: Option<NativeDisplayPixelGeometry>,
    foreground_text_hsb: Option<NativeInactivePaneHsb>,
    text_background_opacity: Option<NativeTextBackgroundOpacity>,
    window_background_opacity: Option<NativeTextBackgroundOpacity>,
    background: Option<Vec<NativeWindowBackgroundVisualLayer>>,
    window_background_image: Option<String>,
    window_background_image_hsb: Option<NativeInactivePaneHsb>,
    window_background_gradient: Option<NativeWindowBackgroundGradient>,
    window_background_images: Option<Vec<NativeWindowBackgroundImage>>,
    window_background_layers: Option<Vec<NativeWindowBackgroundVisualLayer>>,
    kde_window_background_blur: Option<bool>,
    macos_window_background_blur: Option<u32>,
    win32_system_backdrop: Option<NativeWin32SystemBackdrop>,
    next: NativeWindowConfigPatchValues1,
}

impl Deref for NativeWindowConfigPatchValues {
    type Target = NativeWindowConfigPatchValues1;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeWindowConfigPatchValues {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeWindowConfigPatchValues1 {
    win32_acrylic_accent_color: Option<Color>,
    window_frame_appearance: Option<NativeWindowFrameAppearance>,
    inactive_pane_hsb: Option<NativeInactivePaneHsb>,
    tab_max_width: Option<usize>,
    tab_min_width: Option<usize>,
    status_update_interval_ms: Option<u64>,
    max_fps: Option<usize>,
    animation_fps: Option<usize>,
    front_end: Option<NativeRenderFrontEnd>,
    webgpu_power_preference: Option<NativeWebGpuPowerPreference>,
    webgpu_force_fallback_adapter: Option<bool>,
    webgpu_preferred_adapter: Option<NativeWebGpuPreferredAdapter>,
    prefer_egl: Option<bool>,
    enable_wayland: Option<bool>,
    enable_zwlr_output_manager: Option<bool>,
    use_box_model_render: Option<bool>,
    experimental_pixel_positioning: Option<bool>,
    shape_cache_size: Option<usize>,
    line_state_cache_size: Option<usize>,
    line_quad_cache_size: Option<usize>,
    line_to_ele_shape_cache_size: Option<usize>,
    glyph_cache_image_cache_size: Option<usize>,
    cursor_blink_rate_ms: Option<u64>,
    cursor_blink_ease_in: Option<NativeEasingFunction>,
    cursor_blink_ease_out: Option<NativeEasingFunction>,
    text_blink_rate_ms: Option<u64>,
    text_blink_rate_rapid_ms: Option<u64>,
    text_blink_ease_in: Option<NativeEasingFunction>,
    text_blink_ease_out: Option<NativeEasingFunction>,
    text_blink_rapid_ease_in: Option<NativeEasingFunction>,
    text_blink_rapid_ease_out: Option<NativeEasingFunction>,
    bold_brightens_ansi_colors: Option<NativeBoldBrightensAnsiColors>,
    default_cursor_style: Option<NativeCursorStyle>,
    cursor_thickness: Option<NativeCursorThickness>,
    underline_thickness: Option<NativeUnderlineThickness>,
    underline_position: Option<NativeUnderlinePosition>,
    strikethrough_position: Option<NativeStrikethroughPosition>,
    force_reverse_video_cursor: Option<bool>,
    reverse_video_cursor_min_contrast: Option<NativeContrastRatio>,
    text_min_contrast_ratio: Option<NativeTextMinContrastRatio>,
    window_decorations: Option<NativeWindowDecorations>,
    integrated_title_buttons: Option<Vec<NativeIntegratedTitleButton>>,
    integrated_title_button_alignment: Option<NativeIntegratedTitleButtonAlignment>,
    integrated_title_button_color: Option<NativeIntegratedTitleButtonColor>,
    next: NativeWindowConfigPatchValues2,
}

impl Deref for NativeWindowConfigPatchValues1 {
    type Target = NativeWindowConfigPatchValues2;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeWindowConfigPatchValues1 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeWindowConfigPatchValues2 {
    integrated_title_button_style: Option<NativeIntegratedTitleButtonStyle>,
    window_padding: Option<NativeWindowPadding>,
    window_content_alignment: Option<NativeWindowContentAlignment>,
    initial_cols: Option<u16>,
    initial_rows: Option<u16>,
    adjust_window_size_when_changing_font_size: Option<bool>,
    command_palette_rows: Option<usize>,
    command_palette_font: Option<NativeFontConfig>,
    command_palette_font_size: Option<NativeFontSize>,
    command_palette_bg_color: Option<Color>,
    command_palette_fg_color: Option<Color>,
    char_select_font: Option<NativeFontConfig>,
    char_select_font_size: Option<NativeFontSize>,
    char_select_bg_color: Option<Color>,
    char_select_fg_color: Option<Color>,
    pane_select_font: Option<NativeFontConfig>,
    pane_select_font_size: Option<NativeFontSize>,
    pane_select_bg_color: Option<Color>,
    pane_select_fg_color: Option<Color>,
    launcher_alphabet: Option<String>,
    quick_select_alphabet: Option<String>,
    quick_select_patterns: Option<Vec<String>>,
    disable_default_quick_select_patterns: Option<bool>,
    quick_select_remove_styling: Option<bool>,
    hyperlink_rules: Option<Vec<NativeHyperlinkRule>>,
    selection_word_boundary: Option<String>,
    default_prog: Option<Vec<String>>,
    default_domain: Option<String>,
    prefer_to_spawn_tabs: Option<bool>,
    set_environment_variables: Option<BTreeMap<String, String>>,
    default_gui_startup_args: Option<Vec<String>>,
    default_workspace: Option<String>,
    native_macos_fullscreen_mode: Option<bool>,
    macos_fullscreen_extend_behind_notch: Option<bool>,
    use_resize_increments: Option<bool>,
    default_cwd: Option<String>,
    default_ssh_auth_sock: Option<String>,
    default_mux_server_domain: Option<String>,
    daemon_options: Option<NativeDaemonOptions>,
    exec_domains: Option<Vec<NativeExecDomain>>,
    wsl_domains: Option<Vec<NativeWslDomain>>,
    unix_domains: Option<Vec<NativeUnixDomain>>,
    ssh_domains: Option<Vec<NativeSshDomain>>,
    next: NativeWindowConfigPatchValues3,
}

impl Deref for NativeWindowConfigPatchValues2 {
    type Target = NativeWindowConfigPatchValues3;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeWindowConfigPatchValues2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[expect(
    clippy::option_option,
    reason = "nested options distinguish absent, explicit nil, and concrete values"
)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeWindowConfigPatchValues3 {
    tls_servers: Option<Vec<NativeTlsServerDomain>>,
    tls_clients: Option<Vec<NativeTlsClientDomain>>,
    serial_ports: Option<Vec<NativeSerialDomain>>,
    mux_enable_ssh_agent: Option<bool>,
    ssh_backend: Option<NativeSshBackend>,
    ratelimit_mux_line_prefetches_per_second: Option<u32>,
    mux_output_parser_buffer_size: Option<usize>,
    mux_output_parser_coalesce_delay_ms: Option<u64>,
    mux_env_remove: Option<Vec<String>>,
    periodic_stat_logging: Option<u64>,
    ulimit_nofile: Option<u64>,
    ulimit_nproc: Option<u64>,
    tiling_desktop_environments: Option<Vec<String>>,
    launch_menu: Option<Vec<NativeLaunchMenuItem>>,
    term: Option<String>,
    enq_answerback: Option<String>,
    audible_bell: Option<NativeAudibleBell>,
    visual_bell: Option<NativeVisualBell>,
    visual_bell_color: Option<Color>,
    notification_handling: Option<NativeNotificationHandling>,
    colors: Option<Box<NativePalette>>,
    color_scheme: Option<String>,
    color_scheme_dirs: Option<Vec<String>>,
    color_schemes: Option<HashMap<String, NativeResolvedPalette>>,
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
    selection_fg_color: Option<Option<Color>>,
    selection_bg_color: Option<Color>,
    cursor_bg_color: Option<Color>,
    cursor_border_color: Option<Color>,
    cursor_fg_color: Option<Color>,
    compose_cursor_color: Option<Color>,
    split_color: Option<Color>,
    scrollbar_thumb_color: Option<Color>,
    tab_bar_background_color: Option<Color>,
    tab_bar_inactive_tab_edge_color: Option<Color>,
    tab_bar_active_tab_colors: Option<NativeTabBarItemColors>,
    tab_bar_inactive_tab_colors: Option<NativeTabBarItemColors>,
    tab_bar_inactive_tab_hover_colors: Option<NativeTabBarItemColors>,
    tab_bar_new_tab_colors: Option<NativeTabBarItemColors>,
    tab_bar_new_tab_hover_colors: Option<NativeTabBarItemColors>,
    next: NativeWindowConfigPatchValues4,
}

impl Deref for NativeWindowConfigPatchValues3 {
    type Target = NativeWindowConfigPatchValues4;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeWindowConfigPatchValues3 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeWindowConfigPatchValues4 {
    tab_bar_style: Option<NativeTabBarStyle>,
    copy_mode_active_highlight_fg: Option<NativeColorSpec>,
    copy_mode_active_highlight_bg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_fg: Option<NativeColorSpec>,
    copy_mode_inactive_highlight_bg: Option<NativeColorSpec>,
    quick_select_label_fg: Option<NativeColorSpec>,
    quick_select_label_bg: Option<NativeColorSpec>,
    quick_select_match_fg: Option<NativeColorSpec>,
    quick_select_match_bg: Option<NativeColorSpec>,
    input_selector_label_fg: Option<NativeColorSpec>,
    input_selector_label_bg: Option<NativeColorSpec>,
    launcher_label_fg: Option<NativeColorSpec>,
    launcher_label_bg: Option<NativeColorSpec>,
    automatically_reload_config: Option<bool>,
    check_for_updates: Option<bool>,
    check_for_updates_interval_seconds: Option<u64>,
    show_update_window: Option<bool>,
    key_map_preference: Option<NativeKeyMapPreference>,
    ui_key_cap_rendering: Option<NativeUiKeyCapRendering>,
    swap_backspace_and_delete: Option<bool>,
    enable_kitty_graphics: Option<bool>,
    enable_checksum_rectangular_area: Option<bool>,
    enable_title_reporting: Option<bool>,
    enable_csi_u_key_encoding: Option<bool>,
    enable_kitty_keyboard: Option<bool>,
    allow_download_protocols: Option<bool>,
    xcursor_theme: Option<String>,
    xcursor_size: Option<u32>,
    palette_max_key_assigments_for_action: Option<usize>,
    allow_win32_input_mode: Option<bool>,
    treat_left_ctrlalt_as_altgr: Option<bool>,
    send_composed_key_when_left_alt_is_pressed: Option<bool>,
    send_composed_key_when_right_alt_is_pressed: Option<bool>,
    treat_east_asian_ambiguous_width_as_wide: Option<bool>,
    normalize_output_to_unicode_nfc: Option<bool>,
    unicode_version: Option<u32>,
    bidi_enabled: Option<bool>,
    bidi_direction: Option<NativeBidiDirection>,
    use_ime: Option<bool>,
    use_dead_keys: Option<bool>,
    ime_preedit_rendering: Option<NativeImePreeditRendering>,
    macos_forward_to_ime_modifier_mask: Option<ModifiersState>,
    xim_im_name: Option<String>,
    next: NativeWindowConfigPatchValues5,
}

impl Deref for NativeWindowConfigPatchValues4 {
    type Target = NativeWindowConfigPatchValues5;

    fn deref(&self) -> &Self::Target {
        &self.next
    }
}

impl DerefMut for NativeWindowConfigPatchValues4 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.next
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NativeWindowConfigPatchValues5 {
    detect_password_input: Option<bool>,
    canonicalize_pasted_newlines: Option<NativeCanonicalizePastedNewlines>,
    quote_dropped_files: Option<NativeQuoteDroppedFiles>,
    alternate_buffer_wheel_scroll_speed: Option<usize>,
    bypass_mouse_reporting_modifiers: Option<ModifiersState>,
    enable_scroll_bar: Option<bool>,
    scrollback_lines: Option<usize>,
    min_scroll_bar_height: Option<NativeScrollBarHeight>,
    unzoom_on_switch_pane: Option<bool>,
    scroll_to_bottom_on_input: Option<bool>,
    disable_default_key_bindings: Option<bool>,
    disable_default_mouse_bindings: Option<bool>,
    hide_mouse_cursor_when_typing: Option<bool>,
    pane_focus_follows_mouse: Option<bool>,
    swallow_mouse_click_on_pane_focus: Option<bool>,
    swallow_mouse_click_on_window_focus: Option<bool>,
    debug_key_events: Option<bool>,
    log_unknown_escape_sequences: Option<bool>,
    warn_about_missing_glyphs: Option<bool>,
    leader: Option<NativeLeaderKey>,
    key_assignments: Option<Vec<NativeUserKeyAssignment>>,
    key_tables: Option<BTreeMap<String, Vec<NativeUserKeyAssignment>>>,
    mouse_assignments: Option<Vec<NativeUserMouseAssignment>>,
    enable_tab_bar: Option<bool>,
    hide_tab_bar_if_only_one_tab: Option<bool>,
    use_fancy_tab_bar: Option<bool>,
    tab_bar_at_bottom: Option<bool>,
    tab_and_split_indices_are_zero_based: Option<bool>,
    mouse_wheel_scrolls_tabs: Option<bool>,
    switch_to_last_active_tab_when_closing_tab: Option<bool>,
    tab_shortcut_style: Option<NativeTabShortcutStyle>,
    closed_tab_history_size: Option<usize>,
    close_tab_selection: Option<CloseTabSelection>,
    tab_bar_wheel_behavior: Option<NativeTabBarWheelBehavior>,
    quit_when_all_windows_are_closed: Option<bool>,
    window_close_confirmation: Option<NativeWindowCloseConfirmation>,
    exit_behavior: Option<NativeExitBehavior>,
    clean_exit_codes: Option<Vec<u32>>,
    exit_behavior_messaging: Option<NativeExitBehaviorMessaging>,
    skip_close_confirmation_for_processes_named: Option<Vec<String>>,
    show_close_tab_button_in_tabs: Option<bool>,
    show_new_tab_button_in_tab_bar: Option<bool>,
    show_tab_index_in_tab_bar: Option<bool>,
    show_tabs_in_tab_bar: Option<bool>,
}
