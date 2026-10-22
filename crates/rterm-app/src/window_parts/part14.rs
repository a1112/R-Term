fn detach_domain_from_query(query: &str) -> Option<WindowDomainSelector> {
    detach_domain_from_query_with_static_source(None, query)
}
fn detach_domain_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowDomainSelector> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(domain) = strip_lua_function_call_from_query(query, "detachdomain") {
        if (domain.trim_start().starts_with('{') || static_source.is_some())
            && let Some(selector) = window_domain_selector_lua_table_from_query_with_static_source(
                static_source,
                domain,
            )
        {
            return Some(selector);
        }
        return window_domain_selector_from_query_with_static_source(static_source, domain);
    }

    if let Some(domain) = strip_query_table_assignment_from_prefix(query, "detachdomain=")
        && (domain.trim_start().starts_with('{') || static_source.is_some())
        && let Some(selector) =
            window_domain_selector_lua_table_from_query_with_static_source(static_source, domain)
    {
        return Some(selector);
    }

    let domain = strip_query_prefix_from_any(
        query,
        &[
            "detach domain=",
            "detach domain ",
            "detachdomain=",
            "detachdomain ",
        ],
    )?;
    if domain.trim_start().starts_with('{') {
        return window_domain_selector_lua_table_from_query_with_static_source(
            static_source,
            domain,
        );
    }
    window_domain_selector_from_query_with_static_source(static_source, domain)
}

fn named_domain_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    domain: &str,
) -> Option<String> {
    let domain = strip_query_prefix_from_any(
        domain,
        &[
            "domain name=",
            "domain name ",
            "domain=",
            "domain ",
            "name=",
            "name ",
        ],
    )
    .unwrap_or(domain);
    parse_maybe_static_query_text(static_source, domain).filter(|domain| !domain.is_empty())
}

fn window_domain_selector_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    domain: &str,
) -> Option<WindowDomainSelector> {
    let domain = named_domain_from_query_with_static_source(static_source, domain)?;
    let normalized = domain
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "currentpanedomain" | "currentpane" | "current" => {
            Some(WindowDomainSelector::CurrentPaneDomain)
        }
        "defaultdomain" | "default" => Some(WindowDomainSelector::DefaultDomain),
        _ => Some(WindowDomainSelector::DomainName(domain)),
    }
}

fn window_domain_selector_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowDomainSelector> {
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
            if value.is_empty() {
                return None;
            }
            domain = Some(WindowDomainSelector::DomainName(value));
        } else if key.eq_ignore_ascii_case("domainid") {
            domain = Some(WindowDomainSelector::DomainId(
                parse_maybe_static_usize_query(static_source, value)?,
            ));
        } else {
            return None;
        }
    }
    domain
}

fn spawn_command_environment_from_query(environment: &str) -> Result<(String, String), ()> {
    let (name, value) = environment.split_once('=').ok_or(())?;
    let name = non_empty_spawn_command_option_value(name)?;
    Ok((name, value.to_owned()))
}

fn non_empty_spawn_command_option_value(value: &str) -> Result<String, ()> {
    if value.is_empty() {
        Err(())
    } else {
        Ok(value.to_owned())
    }
}

fn spawn_command_window_position_from_query(position: &str) -> Result<WindowPosition, ()> {
    let (origin, coordinates) =
        if let Some(coordinates) = strip_query_prefix_from_any(position, &["screen:"]) {
            (WindowPositionOrigin::Screen, coordinates)
        } else if let Some(coordinates) = strip_query_prefix_from_any(position, &["main:"]) {
            (WindowPositionOrigin::Main, coordinates)
        } else if let Some(coordinates) = strip_query_prefix_from_any(position, &["active:"]) {
            (WindowPositionOrigin::Active, coordinates)
        } else if let Some((monitor, coordinates)) = position.split_once(':') {
            if monitor.is_empty() {
                return Err(());
            }
            (
                WindowPositionOrigin::Monitor(monitor.to_owned()),
                coordinates,
            )
        } else {
            (WindowPositionOrigin::Screen, position)
        };

    if coordinates.contains(':') {
        return Err(());
    }
    let (x, y) = coordinates.split_once(',').ok_or(())?;
    if y.contains(',') {
        return Err(());
    }
    let x = x.parse::<i32>().map_err(|_| ())?;
    let y = y.parse::<i32>().map_err(|_| ())?;
    Ok(WindowPosition { origin, x, y })
}

fn spawn_command_window_position_value_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    position: &str,
) -> Result<WindowPosition, ()> {
    let position = position.trim();
    if position.starts_with('{') {
        return spawn_command_window_position_table_from_query(position);
    }
    let position = parse_maybe_static_query_text(static_source, position).ok_or(())?;
    spawn_command_window_position_from_query(&position)
}

fn spawn_command_window_position_table_from_query(position: &str) -> Result<WindowPosition, ()> {
    let table = position
        .trim()
        .strip_prefix('{')
        .and_then(|table| table.strip_suffix('}'))
        .ok_or(())?
        .trim();
    let mut x = None;
    let mut y = None;
    let mut origin = None;
    for field in split_lua_table_top_level_fields(table).ok_or(())? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = split_lua_table_assignment_from_field(field).ok_or(())?;
        let key = split_lua_table_key_from_query(key.trim()).ok_or(())?;
        let value = value.trim();
        if key.eq_ignore_ascii_case("x") {
            if x.is_some() {
                return Err(());
            }
            x = Some(spawn_command_window_position_coordinate_from_query(value)?);
        } else if key.eq_ignore_ascii_case("y") {
            if y.is_some() {
                return Err(());
            }
            y = Some(spawn_command_window_position_coordinate_from_query(value)?);
        } else if key.eq_ignore_ascii_case("origin") {
            if origin.is_some() {
                return Err(());
            }
            origin = Some(spawn_command_window_position_origin_from_query(value)?);
        } else {
            return Err(());
        }
    }
    Ok(WindowPosition {
        origin: origin.unwrap_or(WindowPositionOrigin::Screen),
        x: x.ok_or(())?,
        y: y.ok_or(())?,
    })
}

fn spawn_command_window_position_coordinate_from_query(value: &str) -> Result<i32, ()> {
    let value = parse_maybe_quoted_query_text(value).ok_or(())?;
    value.parse().map_err(|_| ())
}

fn spawn_command_window_position_origin_from_query(
    value: &str,
) -> Result<WindowPositionOrigin, ()> {
    let value = value.trim();
    if value.starts_with('{') {
        let table = value
            .strip_prefix('{')
            .and_then(|table| table.strip_suffix('}'))
            .ok_or(())?
            .trim();
        let mut monitor = None;
        for field in split_lua_table_top_level_fields(table).ok_or(())? {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            let (key, value) = split_lua_table_assignment_from_field(field).ok_or(())?;
            let key = split_lua_table_key_from_query(key.trim()).ok_or(())?;
            if !key.eq_ignore_ascii_case("named") || monitor.is_some() {
                return Err(());
            }
            let value = parse_maybe_quoted_query_text(value.trim()).ok_or(())?;
            monitor = Some(non_empty_spawn_command_option_value(&value)?);
        }
        return monitor.map(WindowPositionOrigin::Monitor).ok_or(());
    }

    let value = parse_maybe_quoted_query_text(value).ok_or(())?;
    let normalized = value
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "screencoordinatesystem" | "screen" => Ok(WindowPositionOrigin::Screen),
        "mainscreen" | "main" => Ok(WindowPositionOrigin::Main),
        "activescreen" | "active" => Ok(WindowPositionOrigin::Active),
        _ => Err(()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowSearchCommandQuery {
    Pattern {
        pattern: String,
        match_type: WindowSearchMatchType,
    },
    #[allow(dead_code)]
    CurrentSelectionOrEmptyString,
}

fn search_query_from_query(query: &str) -> Option<WindowSearchCommandQuery> {
    search_query_from_query_with_static_source(None, query)
}

fn search_query_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSearchCommandQuery> {
    let indexed_query;
    let query = if let Some(query) = strip_wezterm_action_prefix(query) {
        query
    } else if let Some(query) = strip_wezterm_action_index_prefix(query) {
        indexed_query = query;
        indexed_query.as_str()
    } else {
        query
    };

    if let Some(search_query) =
        search_query_lua_action_from_query_with_static_source(static_source, query)
    {
        return Some(search_query);
    }

    let pattern = strip_query_prefix_from_any(query, &["search=", "search "])?;
    if pattern.is_empty() {
        return None;
    }
    if !(pattern.starts_with('"') || pattern.starts_with('\''))
        && search_current_selection_query_matches(pattern)
    {
        return Some(WindowSearchCommandQuery::CurrentSelectionOrEmptyString);
    }

    let (pattern, match_type) =
        if let Some(pattern) = search_query_strip_match_type_prefix(pattern, &["regex"]) {
            (pattern, WindowSearchMatchType::Regex)
        } else if let Some(pattern) = search_query_strip_match_type_prefix(
            pattern,
            &[
                "case-sensitive",
                "case sensitive",
                "case sensitive string",
                "casesensitivestring",
            ],
        ) {
            (pattern, WindowSearchMatchType::CaseSensitive)
        } else if let Some(pattern) = search_query_strip_match_type_prefix(
            pattern,
            &[
                "case-insensitive",
                "case insensitive",
                "case in sensitive",
                "case insensitive string",
                "case in sensitive string",
                "caseinsensitivestring",
            ],
        ) {
            (pattern, WindowSearchMatchType::CaseInsensitive)
        } else {
            (pattern, WindowSearchMatchType::CaseSensitive)
        };

    let pattern = parse_maybe_static_query_text(static_source, pattern)?;
    Some(WindowSearchCommandQuery::Pattern {
        pattern,
        match_type,
    })
}

fn search_query_lua_action_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    query: &str,
) -> Option<WindowSearchCommandQuery> {
    if let Some(value) = strip_lua_function_call_from_query(query, "search") {
        let value = value.trim();
        if value.starts_with('{') {
            return search_query_lua_table_from_query_with_static_source(static_source, value);
        }
        if static_source.is_some()
            && let Some(search_query) =
                search_query_lua_table_from_query_with_static_source(static_source, value)
        {
            return Some(search_query);
        }
        let value = parse_maybe_static_query_text(static_source, value)?;
        return search_query_lua_string_from_value(&value);
    }

    if let Some(value) = strip_query_prefix_from_any(query, &["search "]) {
        if value.trim_start().starts_with('{') {
            return search_query_lua_table_from_query_with_static_source(static_source, value);
        }
        let value = parse_maybe_static_query_text(static_source, value)?;
        if let Some(search_query) = search_query_lua_string_from_value(&value) {
            return Some(search_query);
        }
    }

    if let Some(value) = strip_query_table_assignment_from_prefix(query, "search=")
        && value.trim_start().starts_with('{')
    {
        return search_query_lua_table_from_query_with_static_source(static_source, value);
    }

    None
}

fn search_query_lua_string_from_value(value: &str) -> Option<WindowSearchCommandQuery> {
    search_current_selection_query_matches(value)
        .then_some(WindowSearchCommandQuery::CurrentSelectionOrEmptyString)
}

fn search_query_lua_table_from_query_with_static_source(
    static_source: Option<LuaStaticSource<'_>>,
    value: &str,
) -> Option<WindowSearchCommandQuery> {
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
    let mut search_query = None;

    for field in split_lua_table_top_level_fields(table)? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (name, value) = split_lua_table_assignment_from_field(field)?;
        let name = split_lua_table_key_from_query_with_static_source(static_source, name.trim())?;
        let value = parse_maybe_static_query_text(static_source, value.trim())?;
        if search_query.is_some() {
            return None;
        }
        let match_type = match normalized_search_lua_field(&name).as_str() {
            "regex" => WindowSearchMatchType::Regex,
            "casesensitivestring" => WindowSearchMatchType::CaseSensitive,
            "caseinsensitivestring" => WindowSearchMatchType::CaseInsensitive,
            _ => return None,
        };
        search_query = Some(WindowSearchCommandQuery::Pattern {
            pattern: value,
            match_type,
        });
    }

    search_query
}

fn normalized_search_lua_field(field: &str) -> String {
    field
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn search_query_strip_match_type_prefix<'a>(
    pattern: &'a str,
    prefixes: &[&str],
) -> Option<&'a str> {
    prefixes.iter().find_map(|prefix| {
        let candidate = pattern.get(..prefix.len())?;
        let rest = pattern.get(prefix.len()..)?;
        candidate
            .eq_ignore_ascii_case(prefix)
            .then_some(rest)
            .and_then(|rest| rest.starts_with(char::is_whitespace).then_some(rest))
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
    })
}

fn search_current_selection_query_matches(pattern: &str) -> bool {
    let normalized = pattern
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "currentselection"
            | "currentselectionorempty"
            | "currentselectionoremptystring"
            | "currentselectionoremptystringpattern"
    )
}

fn single_line_search_query_from_selection(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowQuickSelectOptions {
    patterns: Option<Vec<String>>,
    alphabet: Option<String>,
    label: Option<String>,
    action: Option<WindowQuickSelectAction>,
    skip_action_on_paste: bool,
    scope_lines: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowCharSelectOptions {
    copy_on_select: bool,
    copy_to: WindowCopyDestination,
    group: Option<String>,
}

impl Default for WindowCharSelectOptions {
    fn default() -> Self {
        Self {
            copy_on_select: true,
            copy_to: WindowCopyDestination::ClipboardAndPrimarySelection,
            group: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowCharSelect {
    copy_on_select: bool,
    copy_to: WindowCopyDestination,
    group: Option<String>,
    recently_used: Vec<WindowCharSelectRecent>,
    input: String,
    matches: Vec<WindowCharSelectCandidate>,
    selected: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
struct WindowCharSelectRecent {
    text: String,
    selections: usize,
    #[serde(default)]
    last_used: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct WindowCharSelectRecentStore {
    entries: Vec<WindowCharSelectRecent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowCharSelectCandidate {
    text: String,
    codepoint: u32,
    name: String,
    score: usize,
}

impl WindowCharSelectCandidate {
    fn display_label(&self, selected: bool) -> String {
        format!(
            "{} {} U+{:04X} {}",
            if selected { '>' } else { ' ' },
            self.text,
            self.codepoint,
            self.name
        )
    }
}

impl WindowCharSelect {
    fn from_options(
        options: WindowCharSelectOptions,
        recently_used: &[WindowCharSelectRecent],
    ) -> Self {
        let mut char_select = Self {
            copy_on_select: options.copy_on_select,
            copy_to: options.copy_to,
            group: options
                .group
                .or_else(|| Some(DEFAULT_CHAR_SELECT_GROUP.to_owned())),
            recently_used: recently_used.to_vec(),
            input: String::new(),
            matches: Vec::new(),
            selected: 0,
        };
        char_select.refresh_matches();
        char_select
    }

    fn cycle_group(&mut self, forward: bool) {
        let current = self.group.as_deref().unwrap_or(DEFAULT_CHAR_SELECT_GROUP);
        let index = CHAR_SELECT_GROUPS
            .iter()
            .position(|group| *group == current)
            .unwrap_or_else(|| {
                CHAR_SELECT_GROUPS
                    .iter()
                    .position(|group| *group == DEFAULT_CHAR_SELECT_GROUP)
                    .expect("default char select group is listed")
            });
        let next_index = if forward {
            (index + 1) % CHAR_SELECT_GROUPS.len()
        } else if index == 0 {
            CHAR_SELECT_GROUPS.len() - 1
        } else {
            index - 1
        };
        self.group = Some(CHAR_SELECT_GROUPS[next_index].to_owned());
        self.refresh_matches();
    }

    fn selected_text(&self) -> Option<String> {
        let input = self.input.trim();
        if input.is_empty() && self.group.as_deref() == Some(RECENTLY_USED_CHAR_SELECT_GROUP) {
            return self
                .matches
                .get(self.selected)
                .map(|candidate| candidate.text.clone())
                .or_else(|| self.recently_used.first().map(|recent| recent.text.clone()));
        }
        self.matches
            .get(self.selected)
            .map(|candidate| candidate.text.clone())
    }

    fn refresh_matches(&mut self) {
        let input = self.input.trim();
        self.matches =
            if input.is_empty() && self.group.as_deref() == Some(RECENTLY_USED_CHAR_SELECT_GROUP) {
                char_select_recently_used_candidates(&self.recently_used, CHAR_SELECT_MATCH_LIMIT)
            } else if input.is_empty() {
                char_select_group_candidates(self.group.as_deref(), CHAR_SELECT_MATCH_LIMIT)
            } else {
                char_select_candidates_for_input(&self.input, CHAR_SELECT_MATCH_LIMIT)
            };
        self.selected = 0;
    }

    fn move_selection(&mut self, offset: isize) {
        let len = self.matches.len();
        if len == 0 {
            self.selected = 0;
            return;
        }

        let len = isize::try_from(len).unwrap_or(1);
        let current = isize::try_from(self.selected).unwrap_or(0);
        self.selected = usize::try_from((current + offset).rem_euclid(len)).unwrap_or(0);
    }
}

fn char_select_group_candidates(
    group: Option<&str>,
    limit: usize,
) -> Vec<WindowCharSelectCandidate> {
    if group == Some("NerdFonts") {
        return NERD_FONTS_CHAR_SELECT_CANDIDATES
            .iter()
            .map(|(character, name)| WindowCharSelectCandidate {
                text: character.to_string(),
                codepoint: u32::from(*character),
                name: (*name).to_owned(),
                score: 0,
            })
            .take(limit)
            .collect();
    }

    let candidates = match group.unwrap_or(DEFAULT_CHAR_SELECT_GROUP) {
        "SmileysAndEmotion" => SMILEYS_AND_EMOTION_CHAR_SELECT_CANDIDATES,
        "PeopleAndBody" => PEOPLE_AND_BODY_CHAR_SELECT_CANDIDATES,
        "AnimalsAndNature" => ANIMALS_AND_NATURE_CHAR_SELECT_CANDIDATES,
        "FoodAndDrink" => FOOD_AND_DRINK_CHAR_SELECT_CANDIDATES,
        "TravelAndPlaces" => TRAVEL_AND_PLACES_CHAR_SELECT_CANDIDATES,
        "Activities" => ACTIVITIES_CHAR_SELECT_CANDIDATES,
        "Objects" => OBJECTS_CHAR_SELECT_CANDIDATES,
        "Symbols" => SYMBOLS_CHAR_SELECT_CANDIDATES,
        "Flags" => FLAGS_CHAR_SELECT_CANDIDATES,
        "UnicodeNames" => UNICODE_NAMES_CHAR_SELECT_CANDIDATES,
        _ => &[],
    };

    candidates
        .iter()
        .filter_map(|character| char_select_candidate_for_character(*character, 0))
        .take(limit)
        .collect()
}

fn char_select_recently_used_candidates(
    recently_used: &[WindowCharSelectRecent],
    limit: usize,
) -> Vec<WindowCharSelectCandidate> {
    let mut indexed_recent: Vec<(usize, &WindowCharSelectRecent)> =
        recently_used.iter().enumerate().collect();
    indexed_recent.sort_by_key(|(index, recent)| {
        (
            Reverse(recent.selections),
            Reverse(recent.last_used),
            *index,
        )
    });
    indexed_recent
        .iter()
        .filter_map(|(_, recent)| {
            let mut chars = recent.text.chars();
            let character = chars.next()?;
            chars
                .next()
                .is_none()
                .then_some(character)
                .and_then(|character| char_select_candidate_for_character(character, 0))
        })
        .take(limit)
        .collect()
}

fn char_select_candidates_for_input(input: &str, limit: usize) -> Vec<WindowCharSelectCandidate> {
    let input = input.trim();
    if input.is_empty() || limit == 0 {
        return Vec::new();
    }

    let hex = input
        .strip_prefix("U+")
        .or_else(|| input.strip_prefix("u+"))
        .or_else(|| input.strip_prefix("0x"))
        .or_else(|| input.strip_prefix("0X"))
        .unwrap_or(input);
    if !hex.is_empty()
        && let Ok(codepoint) = u32::from_str_radix(hex, 16)
        && let Some(character) = char::from_u32(codepoint)
    {
        return char_select_candidate_for_character(character, 0)
            .or_else(|| char_select_nerd_font_candidate_for_character(character, 0))
            .map_or_else(Vec::new, |candidate| vec![candidate]);
    }

    if let Some(character) = unicode_names2::character(input)
        && let Some(candidate) = char_select_candidate_for_character(character, 0)
    {
        return vec![candidate];
    }

    let tokens = unicode_name_query_tokens(input);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut matches = char_select_nerd_font_candidates_for_input(&tokens);
    for codepoint in 0..=u32::from(char::MAX) {
        let Some(character) = char::from_u32(codepoint) else {
            continue;
        };
        let Some(name) = unicode_names2::name(character) else {
            continue;
        };
        let name = name.to_string();
        let Some(score) = fuzzy_unicode_name_score(&tokens, &name) else {
            continue;
        };
        matches.push(WindowCharSelectCandidate {
            text: character.to_string(),
            codepoint,
            name,
            score,
        });
    }

    matches.sort_by_key(|candidate| (candidate.score, candidate.codepoint));
    matches.truncate(limit);
    matches
}

fn char_select_nerd_font_candidates_for_input(tokens: &[String]) -> Vec<WindowCharSelectCandidate> {
    NERD_FONTS_CHAR_SELECT_CANDIDATES
        .iter()
        .filter_map(|(character, name)| {
            let score = fuzzy_unicode_name_score(tokens, name)?;
            Some(WindowCharSelectCandidate {
                text: character.to_string(),
                codepoint: u32::from(*character),
                name: (*name).to_owned(),
                score,
            })
        })
        .collect()
}

fn char_select_nerd_font_candidate_for_character(
    character: char,
    score: usize,
) -> Option<WindowCharSelectCandidate> {
    NERD_FONTS_CHAR_SELECT_CANDIDATES
        .iter()
        .find(|(candidate, _)| *candidate == character)
        .map(|(character, name)| WindowCharSelectCandidate {
            text: character.to_string(),
            codepoint: u32::from(*character),
            name: (*name).to_owned(),
            score,
        })
}

fn char_select_candidate_for_character(
    character: char,
    score: usize,
) -> Option<WindowCharSelectCandidate> {
    let name = unicode_names2::name(character)?.to_string();
    Some(WindowCharSelectCandidate {
        text: character.to_string(),
        codepoint: u32::from(character),
        name,
        score,
    })
}

fn unicode_name_query_tokens(query: &str) -> Vec<String> {
    query
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_uppercase)
        .collect()
}

fn fuzzy_unicode_name_score(tokens: &[String], name: &str) -> Option<usize> {
    let mut search_from = 0;
    let mut score = name.len();

    for token in tokens {
        let offset = name[search_from..].find(token)?;
        score = score.saturating_add(offset);
        search_from += offset + token.len();
    }

    Some(score)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowPaneSelectOptions {
    mode: WindowPaneSelectMode,
    show_pane_ids: bool,
    alphabet: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowPromptInputLineOptions {
    description: String,
    prompt: Option<String>,
    initial_value: Option<String>,
    action: Option<WindowPromptInputLineAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowPromptInputLine {
    description: String,
    prompt: String,
    input: String,
    action: Option<WindowPromptInputLineAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowPromptInputLineAction {
    RenameActiveTab,
    SwitchToWorkspaceName,
    SendLineText,
    SendLinePaste,
    Command(Box<WindowCommand>),
}

impl WindowPromptInputLine {
    fn from_options(options: WindowPromptInputLineOptions) -> Self {
        Self {
            description: options.description,
            prompt: options.prompt.unwrap_or_else(|| "> ".to_owned()),
            input: options.initial_value.unwrap_or_default(),
            action: options.action,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowInputSelectorOptions {
    title: String,
    choices: Vec<WindowInputSelectorChoice>,
    alphabet: Option<String>,
    description: Option<String>,
    fuzzy_description: Option<String>,
    fuzzy: bool,
    action: Option<WindowInputSelectorAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowInputSelectorChoice {
    label: String,
    id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowInputSelector {
    title: String,
    choices: Vec<WindowInputSelectorChoice>,
    selected: usize,
    query: String,
    shortcut_prefix: String,
    alphabet: String,
    description: String,
    fuzzy_description: String,
    fuzzy: bool,
    started_fuzzy: bool,
    action: Option<WindowInputSelectorAction>,
}

impl WindowInputSelector {
    fn from_options(options: WindowInputSelectorOptions) -> Self {
        let fuzzy = options.fuzzy;
        Self {
            title: options.title,
            choices: options.choices,
            selected: 0,
            query: String::new(),
            shortcut_prefix: String::new(),
            alphabet: options
                .alphabet
                .filter(|alphabet| !alphabet.is_empty())
                .unwrap_or_else(|| DEFAULT_LAUNCHER_ALPHABET.to_owned()),
            description: options.description.unwrap_or_else(|| {
                "Select an item and press Enter = accept, Esc = cancel, / = filter".to_owned()
            }),
            fuzzy_description: options
                .fuzzy_description
                .unwrap_or_else(|| "Fuzzy matching: ".to_owned()),
            fuzzy,
            started_fuzzy: fuzzy,
            action: options.action,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowInputSelectorAction {
    SendIdText,
    SendIdPaste,
    SendLabelText,
    SendLabelPaste,
    SwitchToWorkspace {
        name: WindowInputSelectorValueParam,
        cwd: Option<WindowInputSelectorValueParam>,
    },
    Command(Box<WindowCommand>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowInputSelectorValueParam {
    Id,
    Label,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowConfirmationOptions {
    message: String,
    action: Box<WindowCommand>,
    cancel: Option<Box<WindowCommand>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowEmitEvent {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
#[allow(clippy::struct_excessive_bools)]
struct WindowActivateKeyTable {
    name: String,
    timeout_milliseconds: Option<u64>,
    one_shot: bool,
    replace_current: bool,
    until_unknown: bool,
    prevent_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct WindowActiveKeyTable {
    name: String,
    timeout_milliseconds: Option<u64>,
    one_shot: bool,
    until_unknown: bool,
    prevent_fallback: bool,
    activated_at: Instant,
}

impl From<WindowActivateKeyTable> for WindowActiveKeyTable {
    fn from(value: WindowActivateKeyTable) -> Self {
        Self {
            name: value.name,
            timeout_milliseconds: value.timeout_milliseconds,
            one_shot: value.one_shot,
            until_unknown: value.until_unknown,
            prevent_fallback: value.prevent_fallback,
            activated_at: Instant::now(),
        }
    }
}

impl WindowActiveKeyTable {
    fn is_expired(&self, now: Instant) -> bool {
        let Some(timeout_milliseconds) = self.timeout_milliseconds else {
            return false;
        };
        now.checked_duration_since(self.activated_at)
            .is_some_and(|elapsed| elapsed >= Duration::from_millis(timeout_milliseconds))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowConfirmation {
    message: String,
    action: Box<WindowCommand>,
    cancel: Option<Box<WindowCommand>>,
}

impl WindowConfirmation {
    fn from_options(options: WindowConfirmationOptions) -> Self {
        Self {
            message: options.message,
            action: options.action,
            cancel: options.cancel,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowInputSelectorShortcut {
    Execute(WindowInputSelectorChoice),
    Pending(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowCloseConfirmation {
    target: WindowCloseTarget,
}

impl WindowCloseConfirmation {
    const fn label(&self) -> &'static str {
        match &self.target {
            WindowCloseTarget::Window => "Close Window",
            WindowCloseTarget::Pane(_) => "Close Current Pane",
            WindowCloseTarget::Tab(_) => "Close Current Tab",
            WindowCloseTarget::Tabs(_) => "Close Tabs",
        }
    }

    const fn action(&self, switch_to_last_active: bool) -> Option<AppAction> {
        match &self.target {
            WindowCloseTarget::Pane(pane) => Some(AppAction::ClosePane { pane: *pane }),
            WindowCloseTarget::Tab(tab) => Some(AppAction::CloseTab {
                tab: *tab,
                switch_to_last_active,
            }),
            WindowCloseTarget::Window | WindowCloseTarget::Tabs(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowCloseTarget {
    Window,
    Pane(rterm_core::PaneId),
    Tab(rterm_core::TabId),
    Tabs(Vec<rterm_core::TabId>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WindowShowLauncherArgs {
    flags: WindowShowLauncherFlags,
    title: Option<String>,
    alphabet: Option<String>,
    help_text: Option<String>,
    fuzzy_help_text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct WindowShowLauncherFlags {
    commands: bool,
    domains: bool,
    fuzzy: bool,
    key_assignments: bool,
    launch_menu_items: bool,
    tabs: bool,
    workspaces: bool,
}

impl WindowShowLauncherFlags {
    fn from_pipe_separated(flags: &str) -> Option<Self> {
        let mut parsed = Self::default();
        let mut saw_flag = false;

        for flag in flags.split('|') {
            let flag = flag.trim();
            if flag.is_empty() {
                return None;
            }
            saw_flag = true;
            match normalized_show_launcher_flag(flag).as_str() {
                "commands" => parsed.commands = true,
                "domains" => parsed.domains = true,
                "fuzzy" => parsed.fuzzy = true,
                "keyassignments" => parsed.key_assignments = true,
                "launchmenuitems" => parsed.launch_menu_items = true,
                "tabs" => parsed.tabs = true,
                "workspaces" => parsed.workspaces = true,
                _ => return None,
            }
        }

        saw_flag.then_some(parsed)
    }

    fn default_launcher() -> Self {
        Self {
            commands: false,
            domains: true,
            fuzzy: false,
            key_assignments: false,
            launch_menu_items: true,
            tabs: false,
            workspaces: false,
        }
    }

    #[allow(dead_code)]
    fn commands() -> Self {
        Self {
            commands: true,
            domains: false,
            fuzzy: false,
            key_assignments: false,
            launch_menu_items: false,
            tabs: false,
            workspaces: false,
        }
    }

    #[allow(dead_code)]
    fn domains() -> Self {
        Self {
            commands: false,
            domains: true,
            fuzzy: false,
            key_assignments: false,
            launch_menu_items: false,
            tabs: false,
            workspaces: false,
        }
    }

    #[allow(dead_code)]
    fn fuzzy() -> Self {
        Self {
            commands: false,
            domains: false,
            fuzzy: true,
            key_assignments: false,
            launch_menu_items: false,
            tabs: false,
            workspaces: false,
        }
    }

    #[allow(dead_code)]
    fn key_assignments() -> Self {
        Self {
            commands: false,
            domains: false,
            fuzzy: false,
            key_assignments: true,
            launch_menu_items: false,
            tabs: false,
            workspaces: false,
        }
    }

    #[allow(dead_code)]
    fn launch_menu_items() -> Self {
        Self {
            commands: false,
            domains: false,
            fuzzy: false,
            key_assignments: false,
            launch_menu_items: true,
            tabs: false,
            workspaces: false,
        }
    }

    #[allow(dead_code)]
    fn tabs() -> Self {
        Self {
            commands: false,
            domains: false,
            fuzzy: false,
            key_assignments: false,
            launch_menu_items: false,
            tabs: true,
            workspaces: false,
        }
    }

    #[allow(dead_code)]
    fn workspaces() -> Self {
        Self {
            commands: false,
            domains: false,
            fuzzy: false,
            key_assignments: false,
            launch_menu_items: false,
            tabs: false,
            workspaces: true,
        }
    }
}

fn normalized_show_launcher_flag(flag: &str) -> String {
    flag.chars()
        .filter(|character| !character.is_whitespace() && *character != '-' && *character != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowSplitPaneOptions {
    direction: SplitDirection,
    domain: Option<WindowSpawnTabDomain>,
    command: Option<WindowSpawnCommandQuery>,
    command_options: Option<WindowSpawnCommandQueryOptions>,
    size: Option<WindowSplitPaneSize>,
    top_level: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct WindowSwitchToWorkspaceOptions {
    name: Option<String>,
    command: Option<WindowSpawnCommandQuery>,
    command_options: Option<WindowSpawnCommandQueryOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum WindowSpawnTabDomain {
    CurrentPaneDomain,
    DefaultDomain,
    DomainId(usize),
    DomainName(String),
}

impl WindowSpawnTabDomain {
    fn is_supported_local_domain(&self, default_domain: &str) -> bool {
        match self {
            Self::CurrentPaneDomain => true,
            Self::DefaultDomain => is_local_domain_name(default_domain),
            Self::DomainId(_) => false,
            Self::DomainName(name) => is_local_domain_name(name),
        }
    }
}

impl WindowDomainSelector {
    fn is_supported_local_domain(&self, default_domain: &str) -> bool {
        match self {
            Self::CurrentPaneDomain => true,
            Self::DefaultDomain => is_local_domain_name(default_domain),
            Self::DomainId(_) => false,
            Self::DomainName(name) => {
                if is_local_domain_name(name) {
                    return true;
                }
                matches!(
                    name.chars()
                        .filter(|character| !character.is_whitespace()
                            && *character != '-'
                            && *character != '_')
                        .collect::<String>()
                        .to_ascii_lowercase()
                        .as_str(),
                    "defaultdomain" | "default"
                ) && is_local_domain_name(default_domain)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum WindowSplitPaneSize {
    Cells(u16),
    Percent(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum WindowClearScrollbackMode {
    ScrollbackOnly,
    ScrollbackAndViewport,
}

const WINDOW_SCROLL_PAGE_AMOUNT_SCALE: i32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
struct WindowScrollByPageAmount {
    per_mille: i32,
}

impl WindowScrollByPageAmount {
    #[allow(dead_code)]
    const fn from_per_mille(per_mille: i32) -> Self {
        Self { per_mille }
    }

    fn viewport_lines(self, page_rows: isize) -> isize {
        let page_rows = i128::try_from(page_rows.unsigned_abs()).unwrap_or(i128::MAX);
        let per_mille = i128::from(self.per_mille);
        let scale = i128::from(WINDOW_SCROLL_PAGE_AMOUNT_SCALE);
        let magnitude = (page_rows * per_mille.abs() + (scale / 2)) / scale;
        let signed_lines = if per_mille.is_negative() {
            magnitude
        } else {
            -magnitude
        };

        isize_from_i128_saturating(signed_lines)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct WindowSendKey {
    key: Key,
    modifiers: ModifiersState,
}

impl WindowSendKey {
    fn text(&self) -> Option<String> {
        match self.key.as_ref() {
            Key::Character(character) => Some(character.to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowDomainSelector {
    CurrentPaneDomain,
    DefaultDomain,
    DomainId(usize),
    DomainName(String),
}

fn isize_from_i128_saturating(value: i128) -> isize {
    match isize::try_from(value) {
        Ok(value) => value,
        Err(_) if value.is_negative() => isize::MIN,
        Err(_) => isize::MAX,
    }
}

fn activate_window_relative_index(
    current_index: usize,
    len: usize,
    offset: isize,
    wrap: bool,
) -> Option<usize> {
    if len == 0 || current_index >= len {
        return None;
    }

    let current = i128::try_from(current_index).ok()?;
    let len = i128::try_from(len).ok()?;
    let target = current + (offset as i128);
    if wrap {
        return usize::try_from(target.rem_euclid(len)).ok();
    }
    if !(0..len).contains(&target) {
        return None;
    }
    usize::try_from(target).ok()
}

fn activate_window_absolute_index(index: usize, len: usize) -> Option<usize> {
    (index < len).then_some(index)
}

const fn should_focus_materialized_window(index: usize, len: usize) -> bool {
    len > 0 && index + 1 == len
}

#[expect(
    clippy::large_enum_variant,
    reason = "boxing the compatibility command would add unrelated allocation and API churn"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum WheelAssignmentMatch {
    None,
    DisableDefault,
    Command(WindowCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WheelCommandOutcome {
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WheelCommandClass {
    Viewport,
    Writer,
    PaneUi,
    PaneAction,
    ContextualUi,
    ExplicitFocusOrCreation,
    Global,
    Composite,
    DisableDefault,
    Nop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowCommand {
    ActivateLastTab,
    #[allow(dead_code)]
    ActivateWindow(usize),
    #[allow(dead_code)]
    AttachDomain(String),
    ActivateCommandPalette,
    #[allow(dead_code)]
    ActivateCopyMode,
    #[allow(dead_code)]
    CopyMode(WindowCopyModeAssignment),
    ActivatePaneDown,
    ActivatePaneLeft,
    ActivatePane1,
    ActivatePane2,
    ActivatePane3,
    ActivatePane4,
    ActivatePane5,
    ActivatePane6,
    ActivatePane7,
    ActivatePane8,
    #[allow(dead_code)]
    ActivatePaneByIndex(usize),
    #[allow(dead_code)]
    ActivatePaneDirection(PaneDirection),
    ActivatePaneRight,
    ActivatePaneUp,
    EnterCopyMode,
    ClearScrollback(WindowClearScrollbackMode),
    ClearScrollbackAndViewport,
    ClearSelection,
    CompleteSelection,
    #[allow(dead_code)]
    CompleteSelectionTo(WindowCopyDestination),
    CompleteSelectionOrOpenLinkAtMouseCursor,
    #[allow(dead_code)]
    CompleteSelectionOrOpenLinkAtMouseCursorTo(WindowCopyDestination),
    OpenLinkAtMouseCursor,
    #[allow(dead_code)]
    OpenUri(String),
    CopyToClipboard,
    CopyToPrimarySelection,
    CopyToClipboardAndPrimarySelection,
    #[allow(dead_code)]
    CopyTo(WindowCopyDestination),
    #[allow(dead_code)]
    CopyTextTo {
        text: String,
        destination: WindowCopyDestination,
    },
    #[allow(dead_code)]
    Copy,
    PasteFromClipboard,
    PasteFromPrimarySelection,
    #[allow(dead_code)]
    PasteFrom(WindowPasteSource),
    #[allow(dead_code)]
    Paste,
    #[allow(dead_code)]
    PastePrimarySelection,
    #[allow(dead_code)]
    SendString(String),
    #[allow(dead_code)]
    SendPaste(String),
    #[allow(dead_code)]
    SendKey(WindowSendKey),
    #[allow(dead_code)]
    ScrollByCurrentEventWheelDelta,
    RestartPane,
    InspectPane,
    ReloadConfiguration,
    ToggleFullScreen,
    StartWindowDrag,
    #[allow(dead_code)]
    SetWindowLevel(NativeWindowLevel),
    ToggleAlwaysOnTop,
    ToggleAlwaysOnBottom,
    Show,
    Hide,
    HideApplication,
    QuitApplication,
    DecreaseFontSize,
    #[allow(dead_code)]
    DetachDomain(WindowDomainSelector),
    IncreaseFontSize,
    ResetFontSize,
    ResetFontAndWindowSize,
    ShowDebugOverlay,
    ResetTerminal,
    EnterQuickSelect,
    #[allow(dead_code)]
    QuickSelect(WindowQuickSelectOptions),
    #[allow(dead_code)]
    QuickSelectArgs(WindowQuickSelectOptions),
    #[allow(dead_code)]
    PaneSelect(WindowPaneSelectOptions),
    #[allow(dead_code)]
    PromptInputLine(WindowPromptInputLineOptions),
    #[allow(dead_code)]
    InputSelector(WindowInputSelectorOptions),
    #[allow(dead_code)]
    Confirmation(WindowConfirmationOptions),
    #[allow(dead_code)]
    EmitEvent(WindowEmitEvent),
    #[allow(dead_code)]
    ActivateKeyTable(WindowActivateKeyTable),
    #[allow(dead_code)]
    PopKeyTable,
    #[allow(dead_code)]
    ClearKeyTableStack,
    #[allow(dead_code)]
    Multiple(Vec<WindowCommand>),
    #[allow(dead_code)]
    DisableDefaultAssignment,
    #[allow(dead_code)]
    Nop,
    EnterPaneSelect,
    EnterPaneSelectShowPaneIds,
    EnterPaneSwap,
    EnterPaneSwapKeepFocus,
    EnterPaneMoveToNewTab,
    EnterPaneMoveToNewWindow,
    ShowTabNavigator,
    ShowLauncher,
    #[allow(dead_code)]
    ShowLauncherArgs(WindowShowLauncherArgs),
    #[allow(dead_code)]
    Search(WindowSearchCommandQuery),
    EnterSearch,
    CharSelect,
    #[allow(dead_code)]
    CharSelectArgs(WindowCharSelectOptions),
    #[allow(dead_code)]
    CloseCurrentPane {
        confirm: bool,
    },
    ClosePane,
    CloseWorkspace,
    #[allow(dead_code)]
    CloseCurrentTab {
        confirm: bool,
    },
    CloseTab,
    DuplicateTab,
    ReopenClosedTab,
    CloseOtherTabs,
    CloseTabsToRight,
    #[allow(dead_code)]
    MoveTabToWindow(rterm_core::WindowId),
    MoveTabToNewWindow,
    ActivateTabId(rterm_core::TabId),
    ActivateTab1,
    ActivateTab2,
    ActivateTab3,
    ActivateTab4,
    ActivateTab5,
    ActivateTab6,
    ActivateTab7,
    ActivateTab8,
    ActivateTab9,
    #[allow(dead_code)]
    ActivateTab(isize),
    #[allow(dead_code)]
    ActivateTabRelative(isize),
    #[allow(dead_code)]
    ActivateTabRelativeNoWrap(isize),
    #[allow(dead_code)]
    ActivateWindowRelative(isize),
    #[allow(dead_code)]
    ActivateWindowRelativeNoWrap(isize),
    MoveTabRelativeLeft,
    MoveTabRelativeRight,
    #[allow(dead_code)]
    MoveTabRelative(isize),
    #[allow(dead_code)]
    MoveTab(usize),
    MoveTabTo1,
    MoveTabTo2,
    MoveTabTo3,
    MoveTabTo4,
    MoveTabTo5,
    MoveTabTo6,
    MoveTabTo7,
    MoveTabTo8,
    NextPane,
    PreviousPane,
    NextTab,
    NextTabNoWrap,
    PreviousTab,
    PreviousTabNoWrap,
    #[allow(dead_code)]
    SpawnTab(WindowSpawnTabDomain),
    #[allow(dead_code)]
    SpawnCommandInNewTab(WindowSpawnCommandQuery),
    #[allow(dead_code)]
    SpawnCommandOptionsInNewTab(WindowSpawnCommandQueryOptions),
    #[allow(dead_code)]
    SpawnCommandInNewWindow(WindowSpawnCommandQuery),
    #[allow(dead_code)]
    SpawnCommandOptionsInNewWindow(WindowSpawnCommandQueryOptions),
    NewTab,
    SpawnWindow,
    NewWorkspace,
    RenameTab,
    #[allow(dead_code)]
    RenameTabTo(String),
    RenameWorkspace,
    #[allow(dead_code)]
    RenameWorkspaceTo(String),
    SwitchToWorkspace,
    #[allow(dead_code)]
    SwitchToWorkspaceArgs(WindowSwitchToWorkspaceOptions),
    SwitchToWorkspaceName(String),
    #[allow(dead_code)]
    AdjustPaneSize {
        direction: ResizeDirection,
        amount: u16,
    },
    ResizePaneDown,
    ResizePaneLeft,
    ResizePaneRight,
    ResizePaneUp,
    #[allow(dead_code)]
    RotatePanes(PaneRotationDirection),
    RotatePanesClockwise,
    RotatePanesCounterClockwise,
    #[allow(dead_code)]
    ScrollByLine(isize),
    #[allow(dead_code)]
    ScrollByPage(WindowScrollByPageAmount),
    ScrollLineDown,
    ScrollLineUp,
    ScrollPageDown,
    ScrollPageUp,
    ScrollToBottom,
    ScrollToNextPrompt,
    ScrollToPreviousPrompt,
    #[allow(dead_code)]
    ScrollToPrompt(isize),
    ScrollToTop,
    SelectTextAtMouseCursorCell,
    SelectTextAtMouseCursorWord,
    SelectTextAtMouseCursorLine,
    SelectTextAtMouseCursorBlock,
    SelectTextAtMouseCursorSemanticZone,
    #[allow(dead_code)]
    SelectTextAtMouseCursor(WindowMouseSelectionMode),
    ExtendSelectionToMouseCursorCell,
    ExtendSelectionToMouseCursorWord,
    ExtendSelectionToMouseCursorLine,
    ExtendSelectionToMouseCursorBlock,
    ExtendSelectionToMouseCursorSemanticZone,
    #[allow(dead_code)]
    ExtendSelectionToMouseCursor(WindowMouseSelectionMode),
    SplitDown,
    SplitHorizontal,
    #[allow(dead_code)]
    SplitPane(WindowSplitPaneOptions),
    SplitRight,
    SplitVertical,
    #[allow(dead_code)]
    SetPaneZoomState(bool),
    TogglePaneZoomState,
    #[allow(dead_code)]
    TogglePaneZoom,
    UnzoomPane,
    ZoomPane,
    NextWorkspace,
    PreviousWorkspace,
    #[allow(dead_code)]
    SwitchWorkspaceRelative(isize),
}

fn wheel_action_io_error(command: &WindowCommand, error: AppShellError) -> io::Error {
    io::Error::other(format!(
        "wheel action '{}' failed: {error:?}",
        command.label()
    ))
}

impl WindowCommand {
    #[allow(clippy::too_many_lines)]
    fn wheel_command_class(&self) -> WheelCommandClass {
        match self {
            Self::ScrollByCurrentEventWheelDelta
            | Self::ScrollByLine(_)
            | Self::ScrollByPage(_)
            | Self::ScrollLineDown
            | Self::ScrollLineUp
            | Self::ScrollPageDown
            | Self::ScrollPageUp
            | Self::ScrollToBottom
            | Self::ScrollToNextPrompt
            | Self::ScrollToPreviousPrompt
            | Self::ScrollToPrompt(_)
            | Self::ScrollToTop => WheelCommandClass::Viewport,

            Self::PasteFromClipboard
            | Self::PasteFromPrimarySelection
            | Self::PasteFrom(_)
            | Self::Paste
            | Self::PastePrimarySelection
            | Self::SendString(_)
            | Self::SendPaste(_)
            | Self::SendKey(_) => WheelCommandClass::Writer,

            Self::ActivateCopyMode
            | Self::CopyMode(_)
            | Self::EnterCopyMode
            | Self::ClearSelection
            | Self::CompleteSelection
            | Self::CompleteSelectionTo(_)
            | Self::CompleteSelectionOrOpenLinkAtMouseCursor
            | Self::CompleteSelectionOrOpenLinkAtMouseCursorTo(_)
            | Self::OpenLinkAtMouseCursor
            | Self::CopyToClipboard
            | Self::CopyToPrimarySelection
            | Self::CopyToClipboardAndPrimarySelection
            | Self::CopyTo(_)
            | Self::Copy
            | Self::SelectTextAtMouseCursorCell
            | Self::SelectTextAtMouseCursorWord
            | Self::SelectTextAtMouseCursorLine
            | Self::SelectTextAtMouseCursorBlock
            | Self::SelectTextAtMouseCursorSemanticZone
            | Self::SelectTextAtMouseCursor(_)
            | Self::ExtendSelectionToMouseCursorCell
            | Self::ExtendSelectionToMouseCursorWord
            | Self::ExtendSelectionToMouseCursorLine
            | Self::ExtendSelectionToMouseCursorBlock
            | Self::ExtendSelectionToMouseCursorSemanticZone
            | Self::ExtendSelectionToMouseCursor(_)
            | Self::EnterQuickSelect
            | Self::QuickSelect(_)
            | Self::QuickSelectArgs(_)
            | Self::Search(_)
            | Self::EnterSearch => WheelCommandClass::PaneUi,

            Self::ClearScrollback(_)
            | Self::ClearScrollbackAndViewport
            | Self::ResetTerminal
            | Self::CloseCurrentPane { .. }
            | Self::ClosePane
            | Self::RestartPane
            | Self::InspectPane
            | Self::AdjustPaneSize { .. }
            | Self::ResizePaneDown
            | Self::ResizePaneLeft
            | Self::ResizePaneRight
            | Self::ResizePaneUp
            | Self::SetPaneZoomState(_)
            | Self::TogglePaneZoomState
            | Self::TogglePaneZoom
            | Self::UnzoomPane
            | Self::ZoomPane => WheelCommandClass::PaneAction,

            Self::PromptInputLine(_)
            | Self::InputSelector(_)
            | Self::Confirmation(_)
            | Self::CharSelect
            | Self::CharSelectArgs(_)
            | Self::EmitEvent(_)
            | Self::OpenUri(_)
            | Self::ActivateCommandPalette
            | Self::ShowLauncher
            | Self::ShowLauncherArgs(_)
            | Self::EnterPaneSwap
            | Self::EnterPaneSwapKeepFocus => WheelCommandClass::ContextualUi,

            Self::PaneSelect(options)
                if matches!(
                    options.mode,
                    WindowPaneSelectMode::SwapWithActive
                        | WindowPaneSelectMode::SwapWithActiveKeepFocus
                ) =>
            {
                WheelCommandClass::ContextualUi
            }

            Self::ActivatePaneDown
            | Self::ActivatePaneLeft
            | Self::ActivatePane1
            | Self::ActivatePane2
            | Self::ActivatePane3
            | Self::ActivatePane4
            | Self::ActivatePane5
            | Self::ActivatePane6
            | Self::ActivatePane7
            | Self::ActivatePane8
            | Self::ActivatePaneByIndex(_)
            | Self::ActivatePaneDirection(_)
            | Self::ActivatePaneRight
            | Self::ActivatePaneUp
            | Self::NextPane
            | Self::PreviousPane
            | Self::SplitDown
            | Self::SplitHorizontal
            | Self::SplitPane(_)
            | Self::SplitRight
            | Self::SplitVertical
            | Self::NewTab
            | Self::SpawnTab(_)
            | Self::SpawnCommandInNewTab(_)
            | Self::SpawnCommandOptionsInNewTab(_)
            | Self::SpawnCommandInNewWindow(_)
            | Self::SpawnCommandOptionsInNewWindow(_)
            | Self::SpawnWindow
            | Self::NewWorkspace
            | Self::SwitchToWorkspace
            | Self::SwitchToWorkspaceArgs(_)
            | Self::SwitchToWorkspaceName(_) => WheelCommandClass::ExplicitFocusOrCreation,

            Self::Multiple(_) => WheelCommandClass::Composite,
            Self::DisableDefaultAssignment => WheelCommandClass::DisableDefault,
            Self::Nop => WheelCommandClass::Nop,

            Self::ActivateLastTab
            | Self::ActivateWindow(_)
            | Self::AttachDomain(_)
            | Self::CopyTextTo { .. }
            | Self::ReloadConfiguration
            | Self::ToggleFullScreen
            | Self::StartWindowDrag
            | Self::SetWindowLevel(_)
            | Self::ToggleAlwaysOnTop
            | Self::ToggleAlwaysOnBottom
            | Self::Show
            | Self::Hide
            | Self::HideApplication
            | Self::QuitApplication
            | Self::DecreaseFontSize
            | Self::DetachDomain(_)
            | Self::IncreaseFontSize
            | Self::ResetFontSize
            | Self::ResetFontAndWindowSize
            | Self::ShowDebugOverlay
            | Self::EnterPaneSelect
            | Self::EnterPaneSelectShowPaneIds
            | Self::EnterPaneMoveToNewTab
            | Self::EnterPaneMoveToNewWindow
            | Self::PaneSelect(_)
            | Self::ShowTabNavigator
            | Self::ActivateKeyTable(_)
            | Self::PopKeyTable
            | Self::ClearKeyTableStack
            | Self::CloseWorkspace
            | Self::CloseCurrentTab { .. }
            | Self::CloseTab
            | Self::DuplicateTab
            | Self::ReopenClosedTab
            | Self::CloseOtherTabs
            | Self::CloseTabsToRight
            | Self::MoveTabToWindow(_)
            | Self::MoveTabToNewWindow
            | Self::ActivateTabId(_)
            | Self::ActivateTab1
            | Self::ActivateTab2
            | Self::ActivateTab3
            | Self::ActivateTab4
            | Self::ActivateTab5
            | Self::ActivateTab6
            | Self::ActivateTab7
            | Self::ActivateTab8
            | Self::ActivateTab9
            | Self::ActivateTab(_)
            | Self::ActivateTabRelative(_)
            | Self::ActivateTabRelativeNoWrap(_)
            | Self::ActivateWindowRelative(_)
            | Self::ActivateWindowRelativeNoWrap(_)
            | Self::MoveTabRelativeLeft
            | Self::MoveTabRelativeRight
            | Self::MoveTabRelative(_)
            | Self::MoveTab(_)
            | Self::MoveTabTo1
            | Self::MoveTabTo2
            | Self::MoveTabTo3
            | Self::MoveTabTo4
            | Self::MoveTabTo5
            | Self::MoveTabTo6
            | Self::MoveTabTo7
            | Self::MoveTabTo8
            | Self::RotatePanes(_)
            | Self::RotatePanesClockwise
            | Self::RotatePanesCounterClockwise
            | Self::NextTab
            | Self::NextTabNoWrap
            | Self::PreviousTab
            | Self::PreviousTabNoWrap
            | Self::RenameTab
            | Self::RenameTabTo(_)
            | Self::RenameWorkspace
            | Self::RenameWorkspaceTo(_)
            | Self::NextWorkspace
            | Self::PreviousWorkspace
            | Self::SwitchWorkspaceRelative(_) => WheelCommandClass::Global,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::ActivateLastTab
            | Self::ActivateCommandPalette
            | Self::ActivatePaneDown
            | Self::ActivatePaneLeft
            | Self::ActivatePane1
            | Self::ActivatePane2
            | Self::ActivatePane3
            | Self::ActivatePane4
            | Self::ActivatePane5
            | Self::ActivatePane6
            | Self::ActivatePane7
            | Self::ActivatePane8
            | Self::ActivatePaneByIndex(_)
            | Self::ActivatePaneDirection(_)
            | Self::ActivatePaneRight
            | Self::ActivatePaneUp
            | Self::ActivateTabId(_)
            | Self::ActivateTab1
            | Self::ActivateTab2
            | Self::ActivateTab3
            | Self::ActivateTab4
            | Self::ActivateTab5
            | Self::ActivateTab6
            | Self::ActivateTab7
            | Self::ActivateTab8
            | Self::ActivateTab9
            | Self::ActivateTab(_)
            | Self::ActivateTabRelative(_)
            | Self::ActivateTabRelativeNoWrap(_)
            | Self::MoveTabRelativeLeft
            | Self::MoveTabRelativeRight
            | Self::MoveTabRelative(_)
            | Self::MoveTab(_)
            | Self::MoveTabTo1
            | Self::MoveTabTo2
            | Self::MoveTabTo3
            | Self::MoveTabTo4
            | Self::MoveTabTo5
            | Self::MoveTabTo6
            | Self::MoveTabTo7
            | Self::MoveTabTo8
            | Self::NextPane
            | Self::PreviousPane
            | Self::NextTab
            | Self::NextTabNoWrap
            | Self::PreviousTab
            | Self::PreviousTabNoWrap => self.navigation_label(),
            _ => self.general_label(),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "command labels remain explicit for stable configuration and palette names"
    )]
    fn navigation_label(&self) -> &'static str {
        if let Some(label) = self.activate_pane_label() {
            return label;
        }

        match self {
            Self::ActivateLastTab => "Activate Last Tab",
            Self::ActivateCommandPalette => "Activate Command Palette",
            Self::CloseCurrentPane { .. } | Self::ClosePane => "Close Current Pane",
            Self::CloseWorkspace => "Close Workspace",
            Self::CloseCurrentTab { .. } | Self::CloseTab => "Close Current Tab",
            Self::DuplicateTab => "Duplicate Tab",
            Self::ReopenClosedTab => "Reopen Closed Tab",
            Self::CloseOtherTabs => "Close Other Tabs",
            Self::CloseTabsToRight => "Close Tabs To Right",
            Self::MoveTabToWindow(_) => "Move Tab To Window",
            Self::MoveTabToNewWindow => "Move Tab To New Window",
            Self::ActivateCopyMode | Self::EnterCopyMode => "Activate Copy Mode",
            Self::CopyMode(_) => "Copy Mode",
            Self::ClearScrollback(_) => "Clear Scrollback",
            Self::ClearScrollbackAndViewport => "Clear Scrollback And Viewport",
            Self::ClearSelection => "Clear Selection",
            Self::CompleteSelection | Self::CompleteSelectionTo(_) => "Complete Selection",
            Self::CompleteSelectionOrOpenLinkAtMouseCursor
            | Self::CompleteSelectionOrOpenLinkAtMouseCursorTo(_) => {
                "Complete Selection Or Open Link At Mouse Cursor"
            }
            Self::OpenLinkAtMouseCursor => "Open Link At Mouse Cursor",
            Self::OpenUri(_) => "Open URI",
            Self::CopyToClipboard | Self::CopyTo(WindowCopyDestination::Clipboard) => {
                "Copy To Clipboard"
            }
            Self::Copy => "Copy",
            Self::CopyToPrimarySelection
            | Self::CopyTo(WindowCopyDestination::PrimarySelection) => "Copy To Primary Selection",
            Self::CopyToClipboardAndPrimarySelection
            | Self::CopyTo(WindowCopyDestination::ClipboardAndPrimarySelection) => {
                "Copy To Clipboard And Primary Selection"
            }
            Self::CopyTextTo { .. } => "Copy Text To",
            Self::PasteFromClipboard | Self::PasteFrom(WindowPasteSource::Clipboard) => {
                "Paste From Clipboard"
            }
            Self::Paste => "Paste",
            Self::PasteFromPrimarySelection
            | Self::PasteFrom(WindowPasteSource::PrimarySelection) => {
                "Paste From Primary Selection"
            }
            Self::PastePrimarySelection => "Paste Primary Selection",
            Self::SendString(_) => "Send String",
            Self::SendPaste(_) => "Send Paste",
            Self::SendKey(_) => "Send Key",
            Self::Confirmation(_) => "Confirmation",
            Self::RestartPane => "Restart Pane",
            Self::InspectPane => "Inspect Pane",
            Self::ReloadConfiguration => "Reload Configuration",
            Self::ToggleFullScreen => "Toggle Full Screen",
            Self::Hide => "Hide",
            Self::HideApplication => "Hide Application",
            Self::QuitApplication => "Quit Application",
            Self::DecreaseFontSize => "Decrease Font Size",
            Self::IncreaseFontSize => "Increase Font Size",
            Self::ResetFontSize => "Reset Font Size",
            Self::ResetFontAndWindowSize => "Reset Font And Window Size",
            Self::ShowDebugOverlay => "Show Debug Overlay",
            Self::ResetTerminal => "Reset Terminal",
            Self::EnterQuickSelect => "Quick Select",
            Self::EnterPaneSelect => "Pane Select",
            Self::EnterPaneSelectShowPaneIds => "Pane Select Show Pane IDs",
            Self::EnterPaneSwap => "Pane Select Swap With Active",
            Self::EnterPaneSwapKeepFocus => "Pane Select Swap With Active Keep Focus",
            Self::EnterPaneMoveToNewTab => "Pane Select Move To New Tab",
            Self::EnterPaneMoveToNewWindow => "Pane Select Move To New Window",
            Self::ShowTabNavigator => "Show Tab Navigator",
            Self::Search(_) | Self::EnterSearch => "Search",
            Self::CharSelect | Self::CharSelectArgs(_) => "Char Select",
            Self::ActivateTabId(_) | Self::ActivateTab(_) => "Activate Tab",
            Self::ActivateTab1 => "Activate Tab 1",
            Self::ActivateTab2 => "Activate Tab 2",
            Self::ActivateTab3 => "Activate Tab 3",
            Self::ActivateTab4 => "Activate Tab 4",
            Self::ActivateTab5 => "Activate Tab 5",
            Self::ActivateTab6 => "Activate Tab 6",
            Self::ActivateTab7 => "Activate Tab 7",
            Self::ActivateTab8 => "Activate Tab 8",
            Self::ActivateTab9 => "Activate Tab 9",
            Self::ActivateTabRelative(_) => "Activate Tab Relative",
            Self::ActivateTabRelativeNoWrap(_) => "Activate Tab Relative No Wrap",
            Self::MoveTabRelativeLeft => "Move Tab Relative Left",
            Self::MoveTabRelativeRight => "Move Tab Relative Right",
            Self::MoveTabRelative(_) => "Move Tab Relative",
            Self::MoveTab(_) => "Move Tab",
            Self::MoveTabTo1 => "Move Tab To 1",
            Self::MoveTabTo2 => "Move Tab To 2",
            Self::MoveTabTo3 => "Move Tab To 3",
            Self::MoveTabTo4 => "Move Tab To 4",
            Self::MoveTabTo5 => "Move Tab To 5",
            Self::MoveTabTo6 => "Move Tab To 6",
            Self::MoveTabTo7 => "Move Tab To 7",
            Self::MoveTabTo8 => "Move Tab To 8",
            Self::NextPane => "Activate Pane Direction Next",
            Self::PreviousPane => "Activate Pane Direction Previous",
            Self::NextTab => "Next Tab",
            Self::NextTabNoWrap => "Next Tab No Wrap",
            Self::PreviousTab => "Previous Tab",
            Self::PreviousTabNoWrap => "Previous Tab No Wrap",
            _ => unreachable!("navigation label requested for non-navigation command"),
        }
    }

    fn activate_pane_label(&self) -> Option<&'static str> {
        match self {
            Self::ActivatePaneDown => Some("Activate Pane Direction Down"),
            Self::ActivatePaneLeft => Some("Activate Pane Direction Left"),
            Self::ActivatePane1 => Some("Activate Pane By Index 1"),
            Self::ActivatePane2 => Some("Activate Pane By Index 2"),
            Self::ActivatePane3 => Some("Activate Pane By Index 3"),
            Self::ActivatePane4 => Some("Activate Pane By Index 4"),
            Self::ActivatePane5 => Some("Activate Pane By Index 5"),
            Self::ActivatePane6 => Some("Activate Pane By Index 6"),
            Self::ActivatePane7 => Some("Activate Pane By Index 7"),
            Self::ActivatePane8 => Some("Activate Pane By Index 8"),
            Self::ActivatePaneByIndex(_) => Some("Activate Pane By Index"),
            Self::ActivatePaneDirection(_) => Some("Activate Pane Direction"),
            Self::ActivatePaneRight => Some("Activate Pane Direction Right"),
            Self::ActivatePaneUp => Some("Activate Pane Direction Up"),
            _ => None,
        }
    }

    fn pane_size_label(&self) -> Option<&'static str> {
        match self {
            Self::AdjustPaneSize { .. } => Some("Adjust Pane Size"),
            Self::ResizePaneDown => Some("Adjust Pane Size Down"),
            Self::ResizePaneLeft => Some("Adjust Pane Size Left"),
            Self::ResizePaneRight => Some("Adjust Pane Size Right"),
            Self::ResizePaneUp => Some("Adjust Pane Size Up"),
            _ => None,
        }
    }

    fn window_control_label(&self) -> Option<&'static str> {
        Some(match self {
            Self::ReloadConfiguration => "Reload Configuration",
            Self::RestartPane => "Restart Pane",
            Self::InspectPane => "Inspect Pane",
            Self::ToggleFullScreen => "Toggle Full Screen",
            Self::StartWindowDrag => "Start Window Drag",
            Self::ActivateWindow(_) => "Activate Window",
            Self::ActivateWindowRelative(_) => "Activate Window Relative",
            Self::ActivateWindowRelativeNoWrap(_) => "Activate Window Relative No Wrap",
            Self::SetWindowLevel(_) => "Set Window Level",
            Self::ToggleAlwaysOnTop => "Toggle Always On Top",
            Self::ToggleAlwaysOnBottom => "Toggle Always On Bottom",
            Self::Show => "Show",
            Self::Hide => "Hide",
            Self::HideApplication => "Hide Application",
            Self::QuitApplication => "Quit Application",
            Self::DecreaseFontSize => "Decrease Font Size",
            Self::IncreaseFontSize => "Increase Font Size",
            Self::ResetFontSize => "Reset Font Size",
            Self::ResetFontAndWindowSize => "Reset Font And Window Size",
            Self::ShowDebugOverlay => "Show Debug Overlay",
            Self::ResetTerminal => "Reset Terminal",
            Self::CharSelect | Self::CharSelectArgs(_) => "Char Select",
            _ => return None,
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
    )]
    fn general_label(&self) -> &'static str {
        if let Some(label) = self.scrollback_label() {
            return label;
        }
        if let Some(label) = self.pane_size_label() {
            return label;
        }
        if let Some(label) = self.window_control_label() {
            return label;
        }

        match self {
            Self::CloseCurrentPane { .. } | Self::ClosePane => "Close Current Pane",
            Self::CloseWorkspace => "Close Workspace",
            Self::CloseCurrentTab { .. } | Self::CloseTab => "Close Current Tab",
            Self::DuplicateTab => "Duplicate Tab",
            Self::ReopenClosedTab => "Reopen Closed Tab",
            Self::CloseOtherTabs => "Close Other Tabs",
            Self::CloseTabsToRight => "Close Tabs To Right",
            Self::MoveTabToNewWindow => "Move Tab To New Window",
            Self::ActivateCopyMode | Self::EnterCopyMode => "Activate Copy Mode",
            Self::CopyMode(_) => "Copy Mode",
            Self::ClearScrollback(_) => "Clear Scrollback",
            Self::ClearScrollbackAndViewport => "Clear Scrollback And Viewport",
            Self::ClearSelection => "Clear Selection",
            Self::CompleteSelection | Self::CompleteSelectionTo(_) => "Complete Selection",
            Self::CompleteSelectionOrOpenLinkAtMouseCursor
            | Self::CompleteSelectionOrOpenLinkAtMouseCursorTo(_) => {
                "Complete Selection Or Open Link At Mouse Cursor"
            }
            Self::OpenLinkAtMouseCursor => "Open Link At Mouse Cursor",
            Self::OpenUri(_) => "Open URI",
            Self::CopyToClipboard | Self::CopyTo(WindowCopyDestination::Clipboard) => {
                "Copy To Clipboard"
            }
            Self::Copy => "Copy",
            Self::CopyToPrimarySelection
            | Self::CopyTo(WindowCopyDestination::PrimarySelection) => "Copy To Primary Selection",
            Self::CopyToClipboardAndPrimarySelection
            | Self::CopyTo(WindowCopyDestination::ClipboardAndPrimarySelection) => {
                "Copy To Clipboard And Primary Selection"
            }
            Self::CopyTextTo { .. } => "Copy Text To",
            Self::PasteFromClipboard | Self::PasteFrom(WindowPasteSource::Clipboard) => {
                "Paste From Clipboard"
            }
            Self::Paste => "Paste",
            Self::PasteFromPrimarySelection
            | Self::PasteFrom(WindowPasteSource::PrimarySelection) => {
                "Paste From Primary Selection"
            }
            Self::PastePrimarySelection => "Paste Primary Selection",
            Self::SendString(_) => "Send String",
            Self::SendPaste(_) => "Send Paste",
            Self::SendKey(_) => "Send Key",
            Self::QuickSelect(_) | Self::QuickSelectArgs(_) | Self::EnterQuickSelect => {
                "Quick Select"
            }
            Self::PaneSelect(_) | Self::EnterPaneSelect => "Pane Select",
            Self::PromptInputLine(_) => "Prompt Input Line",
            Self::InputSelector(_) => "Input Selector",
            Self::Confirmation(_) => "Confirmation",
            Self::EmitEvent(_) => "Emit Event",
            Self::AttachDomain(_) => "Attach Domain",
            Self::DetachDomain(_) => "Detach Domain",
            Self::ActivateKeyTable(_) => "Activate Key Table",
            Self::PopKeyTable => "Pop Key Table",
            Self::ClearKeyTableStack => "Clear Key Table Stack",
            Self::Multiple(_) => "Multiple",
            Self::DisableDefaultAssignment => "Disable Default Assignment",
            Self::Nop => "Nop",
            Self::EnterPaneSelectShowPaneIds => "Pane Select Show Pane IDs",
            Self::EnterPaneSwap => "Pane Select Swap With Active",
            Self::EnterPaneSwapKeepFocus => "Pane Select Swap With Active Keep Focus",
            Self::EnterPaneMoveToNewTab => "Pane Select Move To New Tab",
            Self::EnterPaneMoveToNewWindow => "Pane Select Move To New Window",
            Self::ShowTabNavigator => "Show Tab Navigator",
            Self::ShowLauncher | Self::ShowLauncherArgs(_) => "Show Launcher",
            Self::Search(_) | Self::EnterSearch => "Search",
            Self::RotatePanes(_) => "Rotate Panes",
            Self::RotatePanesClockwise => "Rotate Panes Clockwise",
            Self::RotatePanesCounterClockwise => "Rotate Panes Counter Clockwise",
            Self::SelectTextAtMouseCursorCell => "Select Text At Mouse Cursor Cell",
            Self::SelectTextAtMouseCursorWord => "Select Text At Mouse Cursor Word",
            Self::SelectTextAtMouseCursorLine => "Select Text At Mouse Cursor Line",
            Self::SelectTextAtMouseCursorBlock => "Select Text At Mouse Cursor Block",
            Self::SelectTextAtMouseCursorSemanticZone => {
                "Select Text At Mouse Cursor Semantic Zone"
            }
            Self::SelectTextAtMouseCursor(_) => "Select Text At Mouse Cursor",
            Self::ExtendSelectionToMouseCursorCell => "Extend Selection To Mouse Cursor Cell",
            Self::ExtendSelectionToMouseCursorWord => "Extend Selection To Mouse Cursor Word",
            Self::ExtendSelectionToMouseCursorLine => "Extend Selection To Mouse Cursor Line",
            Self::ExtendSelectionToMouseCursorBlock => "Extend Selection To Mouse Cursor Block",
            Self::ExtendSelectionToMouseCursorSemanticZone => {
                "Extend Selection To Mouse Cursor Semantic Zone"
            }
            Self::ExtendSelectionToMouseCursor(_) => "Extend Selection To Mouse Cursor",
            Self::SetPaneZoomState(_) => "Set Pane Zoom State",
            Self::TogglePaneZoomState | Self::TogglePaneZoom => "Toggle Pane Zoom State",
            Self::UnzoomPane => "Unzoom Pane",
            Self::ZoomPane => "Zoom Pane",
            Self::SpawnTab(_)
            | Self::SpawnCommandInNewTab(_)
            | Self::SpawnCommandOptionsInNewTab(_)
            | Self::NewTab => "New Tab",
            Self::SpawnCommandInNewWindow(_)
            | Self::SpawnCommandOptionsInNewWindow(_)
            | Self::SpawnWindow => "Spawn Window",
            Self::NewWorkspace => "New Workspace",
            Self::RenameTab | Self::RenameTabTo(_) => "Rename Tab",
            Self::RenameWorkspace | Self::RenameWorkspaceTo(_) => "Rename Workspace",
            Self::SwitchToWorkspace
            | Self::SwitchToWorkspaceArgs(_)
            | Self::SwitchToWorkspaceName(_) => "Switch To Workspace",
            Self::SwitchWorkspaceRelative(_) => "Switch Workspace Relative",
            Self::SplitDown | Self::SplitVertical => "Split Vertical",
            Self::SplitPane(options) => match options.direction {
                SplitDirection::Up | SplitDirection::Down => "Split Vertical",
                SplitDirection::Left | SplitDirection::Right => "Split Horizontal",
            },
            Self::SplitRight | Self::SplitHorizontal => "Split Horizontal",
            Self::NextWorkspace => "Next Workspace",
            Self::PreviousWorkspace => "Previous Workspace",
            _ => unreachable!("general label requested for navigation command"),
        }
    }

    fn scrollback_label(&self) -> Option<&'static str> {
        Some(match self {
            Self::ScrollByLine(_) => "Scroll By Line",
            Self::ScrollByPage(_) => "Scroll By Page",
            Self::ScrollByCurrentEventWheelDelta => "Scroll By Current Event Wheel Delta",
            Self::ScrollLineDown => "Scroll By Line Down",
            Self::ScrollLineUp => "Scroll By Line Up",
            Self::ScrollPageDown => "Scroll By Page Down",
            Self::ScrollPageUp => "Scroll By Page Up",
            Self::ScrollToBottom => "Scroll To Bottom",
            Self::ScrollToNextPrompt => "Scroll To Prompt Next",
            Self::ScrollToPreviousPrompt => "Scroll To Prompt Previous",
            Self::ScrollToPrompt(_) => "Scroll To Prompt",
            Self::ScrollToTop => "Scroll To Top",
            _ => return None,
        })
    }
}

fn palette_match_score(label: &str, query: &str) -> Option<(usize, usize)> {
    let label = label.to_ascii_lowercase();
    let query = query.to_ascii_lowercase();

    if query.is_empty() {
        return Some((0, 0));
    }

    let label_bytes = label.as_bytes();
    let query_bytes = query.as_bytes();

    let mut query_index = 0usize;
    let mut start = None;
    let mut end = 0usize;

    for (position, character) in label_bytes.iter().enumerate() {
        if query_index >= query_bytes.len() {
            break;
        }
        if character.eq_ignore_ascii_case(&query_bytes[query_index]) {
            if start.is_none() {
                start = Some(position);
            }
            end = position;
            query_index += 1;
        }
    }

    if query_index < query_bytes.len() {
        return None;
    }

    let start = start.unwrap_or_default();
    let span = (end + 1).saturating_sub(start);
    Some((span, start))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeWindowKeyAssignment {
    keys: &'static str,
    command: WindowCommand,
}

fn native_window_key_assignment_entries() -> Vec<WindowCommandPaletteEntry> {
    NATIVE_WINDOW_KEY_ASSIGNMENTS
        .iter()
        .filter(|assignment| native_window_default_key_assignment_enabled(assignment))
        .map(|assignment| window_key_assignment_entry(assignment.keys, assignment.command.clone()))
        .collect()
}

fn native_window_default_key_assignment_enabled(assignment: &NativeWindowKeyAssignment) -> bool {
    cfg!(target_os = "macos")
        || assignment.keys != "SUPER+H"
        || assignment.command != WindowCommand::HideApplication
}

fn window_key_assignment_entry(keys: &str, command: WindowCommand) -> WindowCommandPaletteEntry {
    let label = window_key_assignment_command_label(keys, &command);
    WindowCommandPaletteEntry::Augmented(NativeCommandPaletteEntry {
        brief: format!("{keys}: {label}"),
        doc: None,
        icon: None,
        key_assignment: Some(keys.to_owned()),
        action: command,
    })
}

fn window_key_assignment_command_label(keys: &str, command: &WindowCommand) -> String {
    if matches!(command, WindowCommand::ActivateTab(_))
        && let Some(digit) = window_key_assignment_trailing_digit(keys)
    {
        return format!("Activate Tab {digit}");
    }

    match command {
        WindowCommand::SpawnCommandInNewTab(command)
        | WindowCommand::SpawnCommandInNewWindow(command) => {
            if let Some(label) = &command.label {
                return label.clone();
            }
        }
        _ => {}
    }

    command.label().to_owned()
}

fn window_key_assignment_trailing_digit(keys: &str) -> Option<char> {
    let key = keys.rsplit('+').next()?.trim();
    if key.len() != 1 {
        return None;
    }

    let digit = key.chars().next()?;
    ('1'..='9').contains(&digit).then_some(digit)
}

fn window_key_assignment_matches_key_event(
    keys: &str,
    key: &Key,
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    window_key_assignment_matches_with_leader(
        keys,
        key,
        Some(physical_key),
        modifiers,
        key_map_preference,
        false,
    )
}

fn window_key_assignment_matches_with_leader(
    keys: &str,
    key: &Key,
    physical_key: Option<PhysicalKey>,
    modifiers: ModifiersState,
    key_map_preference: NativeKeyMapPreference,
    leader_active: bool,
) -> bool {
    let mut parsed_modifiers = ModifiersState::empty();
    let mut parsed_key = None;
    let mut leader_required = false;

    for raw_token in keys.split('+') {
        let token = raw_token.trim();
        if token.is_empty() {
            return false;
        }

        if token.contains('|') && token != "|" {
            for modifier_token in token.split('|') {
                if !window_key_assignment_modifier_matches(
                    modifier_token,
                    &mut parsed_modifiers,
                    &mut leader_required,
                ) {
                    return false;
                }
            }
        } else if !window_key_assignment_modifier_matches(
            token,
            &mut parsed_modifiers,
            &mut leader_required,
        ) {
            if parsed_key.is_some() {
                return false;
            }
            parsed_key = Some(token.to_owned());
        }
    }

    leader_required == leader_active
        && parsed_modifiers == modifiers
        && parsed_key.as_deref().is_some_and(|token| {
            window_key_assignment_key_matches(token, key, physical_key, key_map_preference)
        })
}

fn window_key_assignment_modifier_matches(
    token: &str,
    parsed_modifiers: &mut ModifiersState,
    leader_required: &mut bool,
) -> bool {
    let token = token.trim();
    if token.is_empty() {
        return false;
    }

    match token.to_ascii_uppercase().as_str() {
        "CTRL" | "CONTROL" => parsed_modifiers.insert(ModifiersState::CONTROL),
        "SHIFT" => parsed_modifiers.insert(ModifiersState::SHIFT),
        "ALT" | "OPT" | "META" => parsed_modifiers.insert(ModifiersState::ALT),
        "LEADER" => *leader_required = true,
        "SUPER" | "CMD" | "COMMAND" | "WIN" => {
            parsed_modifiers.insert(ModifiersState::SUPER);
        }
        _ => return false,
    }

    true
}

fn native_modifiers_from_wezterm_lua_config(value: &str) -> Option<ModifiersState> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("NONE") {
        return Some(ModifiersState::empty());
    }

    let mut parsed_modifiers = ModifiersState::empty();
    let mut leader_required = false;
    let mut parsed_any = false;

    for token in value.split(['|', '+']) {
        let token = token.trim();
        if token.is_empty() || token.eq_ignore_ascii_case("NONE") {
            return None;
        }
        if !window_key_assignment_modifier_matches(
            token,
            &mut parsed_modifiers,
            &mut leader_required,
        ) {
            return None;
        }
        parsed_any = true;
    }

    (parsed_any && !leader_required).then_some(parsed_modifiers)
}

fn window_key_assignment_key_matches(
    token: &str,
    key: &Key,
    physical_key: Option<PhysicalKey>,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    let token = token.to_ascii_uppercase();
    if let Some(mapped_token) = token.strip_prefix("MAPPED:") {
        return window_key_assignment_key_matches(
            mapped_token,
            key,
            None,
            NativeKeyMapPreference::Mapped,
        );
    }

    if let Some(physical_token) = token.strip_prefix("PHYS:") {
        return window_key_assignment_physical_identifier_matches(physical_token, physical_key);
    }

    if let Some(raw_token) = token.strip_prefix("RAW:") {
        return window_key_assignment_raw_key_matches(raw_token, physical_key);
    }

    if window_key_assignment_browser_key_matches(&token, key, physical_key) {
        return true;
    }

    match token.as_str() {
        "ENTER" | "RETURN" => matches!(key, Key::Named(NamedKey::Enter)),
        "ESC" | "ESCAPE" => matches!(key, Key::Named(NamedKey::Escape)),
        "TAB" => matches!(key, Key::Named(NamedKey::Tab)),
        "SPACE" => {
            matches!(key, Key::Named(NamedKey::Space))
                || matches!(key, Key::Character(character) if character == " ")
        }
        "BACKSPACE" => matches!(key, Key::Named(NamedKey::Backspace)),
        "INSERT" | "INS" => matches!(key, Key::Named(NamedKey::Insert)),
        "DELETE" | "DEL" => matches!(key, Key::Named(NamedKey::Delete)),
        "HOME" => matches!(key, Key::Named(NamedKey::Home)),
        "END" => matches!(key, Key::Named(NamedKey::End)),
        "PAGEUP" | "PAGE_UP" => matches!(key, Key::Named(NamedKey::PageUp)),
        "PAGEDOWN" | "PAGE_DOWN" => matches!(key, Key::Named(NamedKey::PageDown)),
        "LEFTARROW" | "LEFT_ARROW" | "ARROWLEFT" | "ARROW_LEFT" => {
            matches!(key, Key::Named(NamedKey::ArrowLeft))
        }
        "RIGHTARROW" | "RIGHT_ARROW" | "ARROWRIGHT" | "ARROW_RIGHT" => {
            matches!(key, Key::Named(NamedKey::ArrowRight))
        }
        "UPARROW" | "UP_ARROW" | "ARROWUP" | "ARROW_UP" => {
            matches!(key, Key::Named(NamedKey::ArrowUp))
        }
        "DOWNARROW" | "DOWN_ARROW" | "ARROWDOWN" | "ARROW_DOWN" => {
            matches!(key, Key::Named(NamedKey::ArrowDown))
        }
        "CAPSLOCK" | "CAPS_LOCK" => matches!(key, Key::Named(NamedKey::CapsLock)),
        "SCROLLLOCK" | "SCROLL_LOCK" => matches!(key, Key::Named(NamedKey::ScrollLock)),
        "NUMLOCK" | "NUM_LOCK" => matches!(key, Key::Named(NamedKey::NumLock)),
        "PRINTSCREEN" | "PRINT_SCREEN" => matches!(key, Key::Named(NamedKey::PrintScreen)),
        "PAUSE" => matches!(key, Key::Named(NamedKey::Pause)),
        "MENU" | "CONTEXTMENU" | "CONTEXT_MENU" => {
            matches!(key, Key::Named(NamedKey::ContextMenu))
        }
        "MEDIAPLAY" | "MEDIA_PLAY" => matches!(key, Key::Named(NamedKey::MediaPlay)),
        "MEDIAPAUSE" | "MEDIA_PAUSE" => matches!(key, Key::Named(NamedKey::MediaPause)),
        "MEDIAPLAYPAUSE" | "MEDIA_PLAY_PAUSE" => {
            matches!(key, Key::Named(NamedKey::MediaPlayPause))
        }
        "MEDIANEXTTRACK" | "MEDIA_NEXT_TRACK" | "MEDIATRACKNEXT" | "MEDIA_TRACK_NEXT" => {
            matches!(key, Key::Named(NamedKey::MediaTrackNext))
        }
        "MEDIAPREVTRACK"
        | "MEDIA_PREV_TRACK"
        | "MEDIAPREVIOUSTRACK"
        | "MEDIA_PREVIOUS_TRACK"
        | "MEDIATRACKPREVIOUS"
        | "MEDIA_TRACK_PREVIOUS" => {
            matches!(key, Key::Named(NamedKey::MediaTrackPrevious))
        }
        "MEDIAREWIND" | "MEDIA_REWIND" => matches!(key, Key::Named(NamedKey::MediaRewind)),
        "MEDIASTOP" | "MEDIA_STOP" => matches!(key, Key::Named(NamedKey::MediaStop)),
        "MEDIAFASTFORWARD" | "MEDIA_FAST_FORWARD" => {
            matches!(key, Key::Named(NamedKey::MediaFastForward))
        }
        "MEDIARECORD" | "MEDIA_RECORD" => matches!(key, Key::Named(NamedKey::MediaRecord)),
        "VOLUMEDOWN" | "VOLUME_DOWN" | "AUDIOVOLUMEDOWN" | "AUDIO_VOLUME_DOWN" => {
            matches!(key, Key::Named(NamedKey::AudioVolumeDown))
        }
        "VOLUMEUP" | "VOLUME_UP" | "AUDIOVOLUMEUP" | "AUDIO_VOLUME_UP" => {
            matches!(key, Key::Named(NamedKey::AudioVolumeUp))
        }
        "VOLUMEMUTE" | "VOLUME_MUTE" | "AUDIOVOLUMEMUTE" | "AUDIO_VOLUME_MUTE" => {
            matches!(key, Key::Named(NamedKey::AudioVolumeMute))
        }
        _ if window_key_assignment_physical_key_matches(&token, physical_key) => true,
        _ if token
            .strip_prefix('F')
            .and_then(|number| number.parse::<u8>().ok())
            .is_some_and(|number| window_key_assignment_function_key_matches(number, key)) =>
        {
            true
        }
        _ => window_key_assignment_unprefixed_character_matches(
            &token,
            key,
            physical_key,
            key_map_preference,
        ),
    }
}

fn window_key_assignment_unprefixed_character_matches(
    token: &str,
    key: &Key,
    physical_key: Option<PhysicalKey>,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    match key_map_preference {
        NativeKeyMapPreference::Mapped => {
            matches!(key, Key::Character(character) if character.eq_ignore_ascii_case(token))
        }
        NativeKeyMapPreference::Physical => {
            window_key_assignment_physical_character_matches(token, physical_key)
        }
    }
}

fn window_default_tab_index_for_key(
    key: &Key,
    physical_key: Option<PhysicalKey>,
    key_map_preference: NativeKeyMapPreference,
) -> Option<isize> {
    [
        ("1", "!", 0),
        ("2", "@", 1),
        ("3", "#", 2),
        ("4", "$", 3),
        ("5", "%", 4),
        ("6", "^", 5),
        ("7", "&", 6),
        ("8", "*", 7),
        ("9", "(", -1),
    ]
    .into_iter()
    .find_map(|(digit, shifted, index)| {
        window_default_digit_key_matches(digit, shifted, key, physical_key, key_map_preference)
            .then_some(index)
    })
}

fn window_default_digit_key_matches(
    digit: &str,
    shifted: &str,
    key: &Key,
    physical_key: Option<PhysicalKey>,
    key_map_preference: NativeKeyMapPreference,
) -> bool {
    match key_map_preference {
        NativeKeyMapPreference::Mapped => {
            matches!(key, Key::Character(character) if character == digit || character == shifted)
        }
        NativeKeyMapPreference::Physical => {
            window_key_assignment_physical_character_matches(digit, physical_key)
        }
    }
}

fn window_key_assignment_browser_key_matches(
    token: &str,
    key: &Key,
    physical_key: Option<PhysicalKey>,
) -> bool {
    match token {
        "BROWSERBACK" | "BROWSER_BACK" => {
            matches!(key, Key::Named(NamedKey::BrowserBack))
                || window_key_assignment_physical_key_matches(token, physical_key)
        }
        "BROWSERFORWARD" | "BROWSER_FORWARD" => {
            matches!(key, Key::Named(NamedKey::BrowserForward))
                || window_key_assignment_physical_key_matches(token, physical_key)
        }
        "BROWSERREFRESH" | "BROWSER_REFRESH" => {
            matches!(key, Key::Named(NamedKey::BrowserRefresh))
                || window_key_assignment_physical_key_matches(token, physical_key)
        }
        "BROWSERSTOP" | "BROWSER_STOP" => {
            matches!(key, Key::Named(NamedKey::BrowserStop))
                || window_key_assignment_physical_key_matches(token, physical_key)
        }
        "BROWSERSEARCH" | "BROWSER_SEARCH" => {
            matches!(key, Key::Named(NamedKey::BrowserSearch))
                || window_key_assignment_physical_key_matches(token, physical_key)
        }
        "BROWSERFAVORITES" | "BROWSER_FAVORITES" => {
            matches!(key, Key::Named(NamedKey::BrowserFavorites))
                || window_key_assignment_physical_key_matches(token, physical_key)
        }
        "BROWSERHOME" | "BROWSER_HOME" => {
            matches!(key, Key::Named(NamedKey::BrowserHome))
                || window_key_assignment_physical_key_matches(token, physical_key)
        }
        _ => false,
    }
}

fn window_key_assignment_physical_key_matches(
    token: &str,
    physical_key: Option<PhysicalKey>,
) -> bool {
    let Some(PhysicalKey::Code(code)) = physical_key else {
        return false;
    };

    match token {
        "NUMPAD0" => code == WinitKeyCode::Numpad0,
        "NUMPAD1" => code == WinitKeyCode::Numpad1,
        "NUMPAD2" => code == WinitKeyCode::Numpad2,
        "NUMPAD3" => code == WinitKeyCode::Numpad3,
        "NUMPAD4" => code == WinitKeyCode::Numpad4,
        "NUMPAD5" => code == WinitKeyCode::Numpad5,
        "NUMPAD6" => code == WinitKeyCode::Numpad6,
        "NUMPAD7" => code == WinitKeyCode::Numpad7,
        "NUMPAD8" => code == WinitKeyCode::Numpad8,
        "NUMPAD9" => code == WinitKeyCode::Numpad9,
        "MULTIPLY" => code == WinitKeyCode::NumpadMultiply,
        "ADD" => code == WinitKeyCode::NumpadAdd,
        "SEPARATOR" => code == WinitKeyCode::NumpadComma,
        "SUBTRACT" => code == WinitKeyCode::NumpadSubtract,
        "DECIMAL" => code == WinitKeyCode::NumpadDecimal,
        "DIVIDE" => code == WinitKeyCode::NumpadDivide,
        "BROWSERBACK" | "BROWSER_BACK" => code == WinitKeyCode::BrowserBack,
        "BROWSERFORWARD" | "BROWSER_FORWARD" => code == WinitKeyCode::BrowserForward,
        "BROWSERREFRESH" | "BROWSER_REFRESH" => code == WinitKeyCode::BrowserRefresh,
        "BROWSERSTOP" | "BROWSER_STOP" => code == WinitKeyCode::BrowserStop,
        "BROWSERSEARCH" | "BROWSER_SEARCH" => code == WinitKeyCode::BrowserSearch,
        "BROWSERFAVORITES" | "BROWSER_FAVORITES" => code == WinitKeyCode::BrowserFavorites,
        "BROWSERHOME" | "BROWSER_HOME" => code == WinitKeyCode::BrowserHome,
        _ => false,
    }
}

fn window_key_assignment_physical_identifier_matches(
    token: &str,
    physical_key: Option<PhysicalKey>,
) -> bool {
    window_key_assignment_physical_character_matches(token, physical_key)
        || window_key_assignment_physical_key_matches(token, physical_key)
}

fn window_key_assignment_physical_character_matches(
    token: &str,
    physical_key: Option<PhysicalKey>,
) -> bool {
    let Some(PhysicalKey::Code(code)) = physical_key else {
        return false;
    };

    match token {
        "A" => code == WinitKeyCode::KeyA,
        "B" => code == WinitKeyCode::KeyB,
        "C" => code == WinitKeyCode::KeyC,
        "D" => code == WinitKeyCode::KeyD,
        "E" => code == WinitKeyCode::KeyE,
        "F" => code == WinitKeyCode::KeyF,
        "G" => code == WinitKeyCode::KeyG,
        "H" => code == WinitKeyCode::KeyH,
        "I" => code == WinitKeyCode::KeyI,
        "J" => code == WinitKeyCode::KeyJ,
        "K" => code == WinitKeyCode::KeyK,
        "L" => code == WinitKeyCode::KeyL,
        "M" => code == WinitKeyCode::KeyM,
        "N" => code == WinitKeyCode::KeyN,
        "O" => code == WinitKeyCode::KeyO,
        "P" => code == WinitKeyCode::KeyP,
        "Q" => code == WinitKeyCode::KeyQ,
        "R" => code == WinitKeyCode::KeyR,
        "S" => code == WinitKeyCode::KeyS,
        "T" => code == WinitKeyCode::KeyT,
        "U" => code == WinitKeyCode::KeyU,
        "V" => code == WinitKeyCode::KeyV,
        "W" => code == WinitKeyCode::KeyW,
        "X" => code == WinitKeyCode::KeyX,
        "Y" => code == WinitKeyCode::KeyY,
        "Z" => code == WinitKeyCode::KeyZ,
        "0" => code == WinitKeyCode::Digit0,
        "1" => code == WinitKeyCode::Digit1,
        "2" => code == WinitKeyCode::Digit2,
        "3" => code == WinitKeyCode::Digit3,
        "4" => code == WinitKeyCode::Digit4,
        "5" => code == WinitKeyCode::Digit5,
        "6" => code == WinitKeyCode::Digit6,
        "7" => code == WinitKeyCode::Digit7,
        "8" => code == WinitKeyCode::Digit8,
        "9" => code == WinitKeyCode::Digit9,
        _ => false,
    }
}

fn window_key_assignment_raw_key_matches(token: &str, physical_key: Option<PhysicalKey>) -> bool {
    let Ok(expected) = token.parse::<u32>() else {
        return false;
    };

    let Some(PhysicalKey::Unidentified(native_code)) = physical_key else {
        return false;
    };

    match native_code {
        winit::keyboard::NativeKeyCode::Android(code)
        | winit::keyboard::NativeKeyCode::Xkb(code) => code == expected,
        winit::keyboard::NativeKeyCode::MacOS(code)
        | winit::keyboard::NativeKeyCode::Windows(code) => u32::from(code) == expected,
        winit::keyboard::NativeKeyCode::Unidentified => false,
    }
}

fn window_key_assignment_function_key_matches(number: u8, key: &Key) -> bool {
    matches!(
        (number, key),
        (1, Key::Named(NamedKey::F1))
            | (2, Key::Named(NamedKey::F2))
            | (3, Key::Named(NamedKey::F3))
            | (4, Key::Named(NamedKey::F4))
            | (5, Key::Named(NamedKey::F5))
            | (6, Key::Named(NamedKey::F6))
            | (7, Key::Named(NamedKey::F7))
            | (8, Key::Named(NamedKey::F8))
            | (9, Key::Named(NamedKey::F9))
            | (10, Key::Named(NamedKey::F10))
            | (11, Key::Named(NamedKey::F11))
            | (12, Key::Named(NamedKey::F12))
            | (13, Key::Named(NamedKey::F13))
            | (14, Key::Named(NamedKey::F14))
            | (15, Key::Named(NamedKey::F15))
            | (16, Key::Named(NamedKey::F16))
            | (17, Key::Named(NamedKey::F17))
            | (18, Key::Named(NamedKey::F18))
            | (19, Key::Named(NamedKey::F19))
            | (20, Key::Named(NamedKey::F20))
            | (21, Key::Named(NamedKey::F21))
            | (22, Key::Named(NamedKey::F22))
            | (23, Key::Named(NamedKey::F23))
            | (24, Key::Named(NamedKey::F24))
    )
}

const NATIVE_WINDOW_KEY_ASSIGNMENTS: &[NativeWindowKeyAssignment] = &[
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+T",
        command: WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+T",
        command: WindowCommand::SpawnTab(WindowSpawnTabDomain::CurrentPaneDomain),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+SHIFT+T",
        command: WindowCommand::SpawnTab(WindowSpawnTabDomain::DefaultDomain),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+N",
        command: WindowCommand::SpawnWindow,
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+N",
        command: WindowCommand::SpawnWindow,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+W",
        command: WindowCommand::CloseCurrentTab { confirm: true },
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+W",
        command: WindowCommand::CloseCurrentTab { confirm: true },
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+1",
        command: WindowCommand::ActivateTab(0),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+2",
        command: WindowCommand::ActivateTab(1),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+3",
        command: WindowCommand::ActivateTab(2),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+4",
        command: WindowCommand::ActivateTab(3),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+5",
        command: WindowCommand::ActivateTab(4),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+6",
        command: WindowCommand::ActivateTab(5),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+7",
        command: WindowCommand::ActivateTab(6),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+8",
        command: WindowCommand::ActivateTab(7),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+9",
        command: WindowCommand::ActivateTab(-1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+1",
        command: WindowCommand::ActivateTab(0),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+2",
        command: WindowCommand::ActivateTab(1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+3",
        command: WindowCommand::ActivateTab(2),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+4",
        command: WindowCommand::ActivateTab(3),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+5",
        command: WindowCommand::ActivateTab(4),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+6",
        command: WindowCommand::ActivateTab(5),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+7",
        command: WindowCommand::ActivateTab(6),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+8",
        command: WindowCommand::ActivateTab(7),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+9",
        command: WindowCommand::ActivateTab(-1),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+SHIFT+[",
        command: WindowCommand::ActivateTabRelative(-1),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+SHIFT+]",
        command: WindowCommand::ActivateTabRelative(1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+TAB",
        command: WindowCommand::ActivateTabRelative(-1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+PAGEUP",
        command: WindowCommand::ActivateTabRelative(-1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+P",
        command: WindowCommand::ActivateCommandPalette,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+X",
        command: WindowCommand::ActivateCopyMode,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+SPACE",
        command: WindowCommand::QuickSelect(WindowQuickSelectOptions {
            patterns: None,
            alphabet: None,
            label: None,
            action: None,
            skip_action_on_paste: false,
            scope_lines: None,
        }),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+U",
        command: WindowCommand::CharSelect,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+F",
        command: WindowCommand::Search(WindowSearchCommandQuery::Pattern {
            pattern: String::new(),
            match_type: WindowSearchMatchType::CaseSensitive,
        }),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+F",
        command: WindowCommand::Search(WindowSearchCommandQuery::Pattern {
            pattern: String::new(),
            match_type: WindowSearchMatchType::CaseSensitive,
        }),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+R",
        command: WindowCommand::ReloadConfiguration,
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+R",
        command: WindowCommand::ReloadConfiguration,
    },
    NativeWindowKeyAssignment {
        keys: "ALT+ENTER",
        command: WindowCommand::ToggleFullScreen,
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+M",
        command: WindowCommand::Hide,
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+H",
        command: WindowCommand::HideApplication,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+-",
        command: WindowCommand::DecreaseFontSize,
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+-",
        command: WindowCommand::DecreaseFontSize,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+=",
        command: WindowCommand::IncreaseFontSize,
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+=",
        command: WindowCommand::IncreaseFontSize,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+0",
        command: WindowCommand::ResetFontSize,
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+0",
        command: WindowCommand::ResetFontSize,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+L",
        command: WindowCommand::ShowDebugOverlay,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+K",
        command: WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackOnly),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+K",
        command: WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackOnly),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+C",
        command: WindowCommand::CopyTo(WindowCopyDestination::Clipboard),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+C",
        command: WindowCommand::CopyTo(WindowCopyDestination::Clipboard),
    },
    NativeWindowKeyAssignment {
        keys: "COPY",
        command: WindowCommand::CopyTo(WindowCopyDestination::Clipboard),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+INSERT",
        command: WindowCommand::CopyTo(WindowCopyDestination::PrimarySelection),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+V",
        command: WindowCommand::PasteFrom(WindowPasteSource::Clipboard),
    },
    NativeWindowKeyAssignment {
        keys: "SUPER+V",
        command: WindowCommand::PasteFrom(WindowPasteSource::Clipboard),
    },
    NativeWindowKeyAssignment {
        keys: "PASTE",
        command: WindowCommand::PasteFrom(WindowPasteSource::Clipboard),
    },
    NativeWindowKeyAssignment {
        keys: "SHIFT+INSERT",
        command: WindowCommand::PasteFrom(WindowPasteSource::PrimarySelection),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+Z",
        command: WindowCommand::TogglePaneZoomState,
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+TAB",
        command: WindowCommand::ActivateTabRelative(1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+PAGEDOWN",
        command: WindowCommand::ActivateTabRelative(1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+PAGEUP",
        command: WindowCommand::MoveTabRelative(-1),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+PAGEDOWN",
        command: WindowCommand::MoveTabRelative(1),
    },
    NativeWindowKeyAssignment {
        keys: "SHIFT+PAGEUP",
        command: WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(-1_000)),
    },
    NativeWindowKeyAssignment {
        keys: "SHIFT+PAGEDOWN",
        command: WindowCommand::ScrollByPage(WindowScrollByPageAmount::from_per_mille(1_000)),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+ALT+\"",
        command: WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: SplitDirection::Down,
            domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
            command: None,
            command_options: None,
            size: None,
            top_level: false,
        }),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+ALT+%",
        command: WindowCommand::SplitPane(WindowSplitPaneOptions {
            direction: SplitDirection::Right,
            domain: Some(WindowSpawnTabDomain::CurrentPaneDomain),
            command: None,
            command_options: None,
            size: None,
            top_level: false,
        }),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+ALT+LEFTARROW",
        command: WindowCommand::AdjustPaneSize {
            direction: ResizeDirection::Left,
            amount: 1,
        },
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+ALT+RIGHTARROW",
        command: WindowCommand::AdjustPaneSize {
            direction: ResizeDirection::Right,
            amount: 1,
        },
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+ALT+UPARROW",
        command: WindowCommand::AdjustPaneSize {
            direction: ResizeDirection::Up,
            amount: 1,
        },
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+ALT+DOWNARROW",
        command: WindowCommand::AdjustPaneSize {
            direction: ResizeDirection::Down,
            amount: 1,
        },
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+LEFTARROW",
        command: WindowCommand::ActivatePaneDirection(PaneDirection::Left),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+RIGHTARROW",
        command: WindowCommand::ActivatePaneDirection(PaneDirection::Right),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+UPARROW",
        command: WindowCommand::ActivatePaneDirection(PaneDirection::Up),
    },
    NativeWindowKeyAssignment {
        keys: "CTRL+SHIFT+DOWNARROW",
        command: WindowCommand::ActivatePaneDirection(PaneDirection::Down),
    },
];

const WINDOW_COMMANDS: &[WindowCommand] = &[
    WindowCommand::NewTab,
    WindowCommand::SpawnWindow,
    WindowCommand::ActivateCommandPalette,
    WindowCommand::CloseTab,
    WindowCommand::DuplicateTab,
    WindowCommand::ReopenClosedTab,
    WindowCommand::CloseOtherTabs,
    WindowCommand::CloseTabsToRight,
    WindowCommand::MoveTabToNewWindow,
    WindowCommand::ActivateTab1,
    WindowCommand::ActivateTab2,
    WindowCommand::ActivateTab3,
    WindowCommand::ActivateTab4,
    WindowCommand::ActivateTab5,
    WindowCommand::ActivateTab6,
    WindowCommand::ActivateTab7,
    WindowCommand::ActivateTab8,
    WindowCommand::ActivateTab9,
    WindowCommand::NextTab,
    WindowCommand::PreviousTab,
    WindowCommand::NextTabNoWrap,
    WindowCommand::PreviousTabNoWrap,
    WindowCommand::MoveTabRelativeLeft,
    WindowCommand::MoveTabRelativeRight,
    WindowCommand::MoveTabTo1,
    WindowCommand::MoveTabTo2,
    WindowCommand::MoveTabTo3,
    WindowCommand::MoveTabTo4,
    WindowCommand::MoveTabTo5,
    WindowCommand::MoveTabTo6,
    WindowCommand::MoveTabTo7,
    WindowCommand::MoveTabTo8,
    WindowCommand::ActivateLastTab,
    WindowCommand::RotatePanesClockwise,
    WindowCommand::RotatePanesCounterClockwise,
    WindowCommand::SplitHorizontal,
    WindowCommand::SplitVertical,
    WindowCommand::EnterCopyMode,
    WindowCommand::ClearScrollback(WindowClearScrollbackMode::ScrollbackOnly),
    WindowCommand::ClearScrollbackAndViewport,
    WindowCommand::ClearSelection,
    WindowCommand::SelectTextAtMouseCursorCell,
    WindowCommand::SelectTextAtMouseCursorWord,
    WindowCommand::SelectTextAtMouseCursorLine,
    WindowCommand::SelectTextAtMouseCursorBlock,
    WindowCommand::SelectTextAtMouseCursorSemanticZone,
    WindowCommand::ExtendSelectionToMouseCursorCell,
    WindowCommand::ExtendSelectionToMouseCursorWord,
    WindowCommand::ExtendSelectionToMouseCursorLine,
    WindowCommand::ExtendSelectionToMouseCursorBlock,
    WindowCommand::ExtendSelectionToMouseCursorSemanticZone,
    WindowCommand::CompleteSelection,
    WindowCommand::CompleteSelectionOrOpenLinkAtMouseCursor,
    WindowCommand::OpenLinkAtMouseCursor,
    WindowCommand::CopyToClipboard,
    WindowCommand::CopyToPrimarySelection,
    WindowCommand::CopyToClipboardAndPrimarySelection,
    WindowCommand::PasteFromClipboard,
    WindowCommand::PasteFromPrimarySelection,
    WindowCommand::RestartPane,
    WindowCommand::InspectPane,
    WindowCommand::ReloadConfiguration,
    WindowCommand::ToggleFullScreen,
    WindowCommand::StartWindowDrag,
    WindowCommand::ActivateWindow(0),
    WindowCommand::ActivateWindowRelative(1),
    WindowCommand::ActivateWindowRelativeNoWrap(1),
    WindowCommand::ToggleAlwaysOnTop,
    WindowCommand::ToggleAlwaysOnBottom,
    WindowCommand::Show,
    WindowCommand::Hide,
    WindowCommand::HideApplication,
    WindowCommand::QuitApplication,
    WindowCommand::DecreaseFontSize,
    WindowCommand::IncreaseFontSize,
    WindowCommand::ResetFontSize,
    WindowCommand::ResetFontAndWindowSize,
    WindowCommand::ShowDebugOverlay,
    WindowCommand::ResetTerminal,
    WindowCommand::ScrollToTop,
    WindowCommand::ScrollToBottom,
    WindowCommand::ScrollPageUp,
    WindowCommand::ScrollPageDown,
    WindowCommand::ScrollLineUp,
    WindowCommand::ScrollLineDown,
    WindowCommand::ScrollByCurrentEventWheelDelta,
    WindowCommand::ScrollToPreviousPrompt,
    WindowCommand::ScrollToNextPrompt,
    WindowCommand::EnterQuickSelect,
    WindowCommand::EnterPaneSelect,
    WindowCommand::EnterPaneSelectShowPaneIds,
    WindowCommand::EnterPaneSwap,
    WindowCommand::EnterPaneSwapKeepFocus,
    WindowCommand::EnterPaneMoveToNewTab,
    WindowCommand::EnterPaneMoveToNewWindow,
    WindowCommand::ShowTabNavigator,
    WindowCommand::ShowLauncher,
    WindowCommand::EnterSearch,
    WindowCommand::CharSelect,
    WindowCommand::ClosePane,
    WindowCommand::ActivatePaneLeft,
    WindowCommand::ActivatePaneRight,
    WindowCommand::ActivatePaneUp,
    WindowCommand::ActivatePaneDown,
    WindowCommand::ActivatePane1,
    WindowCommand::ActivatePane2,
    WindowCommand::ActivatePane3,
    WindowCommand::ActivatePane4,
    WindowCommand::ActivatePane5,
    WindowCommand::ActivatePane6,
    WindowCommand::ActivatePane7,
    WindowCommand::ActivatePane8,
    WindowCommand::NextPane,
    WindowCommand::PreviousPane,
    WindowCommand::ResizePaneLeft,
    WindowCommand::ResizePaneRight,
    WindowCommand::ResizePaneUp,
    WindowCommand::ResizePaneDown,
    WindowCommand::TogglePaneZoomState,
    WindowCommand::ZoomPane,
    WindowCommand::UnzoomPane,
    WindowCommand::NewWorkspace,
    WindowCommand::CloseWorkspace,
    WindowCommand::RenameTab,
    WindowCommand::RenameWorkspace,
    WindowCommand::SwitchToWorkspace,
    WindowCommand::NextWorkspace,
    WindowCommand::PreviousWorkspace,
];

#[derive(Clone, Debug, PartialEq, Eq)]
enum WindowCommandPaletteEntry {
    BuiltIn(WindowCommand),
    Contextual {
        command: WindowCommand,
        label: String,
    },
    Augmented(NativeCommandPaletteEntry),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WindowLauncherShortcut {
    Execute(Box<WindowCommandPaletteEntry>),
    Pending(String),
}

impl WindowCommandPaletteEntry {
    fn label(&self) -> &str {
        match self {
            Self::BuiltIn(command) => command.label(),
            Self::Contextual { label, .. } => label,
            Self::Augmented(entry) => &entry.brief,
        }
    }

    fn display_label(
        &self,
        selected: bool,
        ui_key_cap_rendering: NativeUiKeyCapRendering,
    ) -> String {
        let prefix = if selected { '>' } else { ' ' };
        match self {
            Self::BuiltIn(command) => format!("{prefix} {}", command.label()),
            Self::Contextual { label, .. } => format!("{prefix} {label}"),
            Self::Augmented(entry) => {
                let brief = augmented_command_palette_display_brief(entry, ui_key_cap_rendering);
                match entry.doc.as_deref() {
                    Some(doc) if !doc.is_empty() => format!("{prefix} {brief} - {doc}"),
                    _ => format!("{prefix} {brief}"),
                }
            }
        }
    }

    fn into_command(self) -> WindowCommand {
        match self {
            Self::BuiltIn(command) | Self::Contextual { command, .. } => command,
            Self::Augmented(entry) => entry.action,
        }
    }
}

fn augmented_command_palette_display_brief(
    entry: &NativeCommandPaletteEntry,
    ui_key_cap_rendering: NativeUiKeyCapRendering,
) -> String {
    let brief = if let Some(keys) = entry.key_assignment.as_deref() {
        let label = entry
            .brief
            .strip_prefix(keys)
            .and_then(|rest| rest.strip_prefix(": "))
            .unwrap_or(&entry.brief);
        format!(
            "{}: {label}",
            render_ui_key_caps(keys, ui_key_cap_rendering)
        )
    } else {
        entry.brief.clone()
    };
    match entry.icon.as_deref().and_then(nerd_font_icon_for_name) {
        Some(icon) => format!("{icon} {brief}"),
        None => brief,
    }
}

fn render_ui_key_caps(keys: &str, style: NativeUiKeyCapRendering) -> String {
    let parts = keys.split('+').collect::<Vec<_>>();
    if parts.is_empty() {
        return String::new();
    }
    let key_index = parts.len().saturating_sub(1);
    let mut rendered = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let normalized = part.trim();
        let is_key = index == key_index;
        rendered.push(if is_key {
            render_ui_key_cap_key(normalized, style)
        } else {
            render_ui_key_cap_modifier(normalized, style)
        });
    }

    match style {
        NativeUiKeyCapRendering::AppleSymbols => rendered.join(""),
        NativeUiKeyCapRendering::Emacs => rendered.join("-"),
        NativeUiKeyCapRendering::UnixLong
        | NativeUiKeyCapRendering::WindowsLong
        | NativeUiKeyCapRendering::WindowsSymbols => rendered.join("+"),
    }
}

fn render_ui_key_cap_modifier(modifier: &str, style: NativeUiKeyCapRendering) -> String {
    match (modifier.to_ascii_uppercase().as_str(), style) {
        ("CTRL" | "CONTROL", NativeUiKeyCapRendering::Emacs) => "C".to_owned(),
        ("CTRL" | "CONTROL", NativeUiKeyCapRendering::AppleSymbols) => "\u{2303}".to_owned(),
        ("CTRL" | "CONTROL", _) => "Ctrl".to_owned(),
        ("SHIFT", NativeUiKeyCapRendering::Emacs) => "S".to_owned(),
        ("SHIFT", NativeUiKeyCapRendering::AppleSymbols) => "\u{21e7}".to_owned(),
        ("SHIFT", _) => "Shift".to_owned(),
        ("ALT" | "OPT" | "OPTION" | "META", NativeUiKeyCapRendering::Emacs) => "M".to_owned(),
        ("ALT" | "OPT" | "OPTION" | "META", NativeUiKeyCapRendering::AppleSymbols) => {
            "\u{2325}".to_owned()
        }
        ("ALT" | "OPT" | "OPTION", _) => "Alt".to_owned(),
        ("META", _) => "Meta".to_owned(),
        ("SUPER" | "CMD" | "COMMAND", NativeUiKeyCapRendering::AppleSymbols) => {
            "\u{2318}".to_owned()
        }
        ("SUPER" | "CMD" | "COMMAND", NativeUiKeyCapRendering::WindowsLong) => "Win".to_owned(),
        ("SUPER" | "CMD" | "COMMAND", NativeUiKeyCapRendering::WindowsSymbols) => {
            "\u{229e}".to_owned()
        }
        ("SUPER" | "CMD" | "COMMAND", _) => "Super".to_owned(),
        _ => render_ui_key_cap_key(modifier, style),
    }
}

fn render_ui_key_cap_key(key: &str, _style: NativeUiKeyCapRendering) -> String {
    match key.to_ascii_uppercase().as_str() {
        "SPACE" => "Space".to_owned(),
        "TAB" => "Tab".to_owned(),
        "ENTER" | "RETURN" => "Enter".to_owned(),
        "ESC" | "ESCAPE" => "Escape".to_owned(),
        "BACKSPACE" => "Backspace".to_owned(),
        "DELETE" => "Delete".to_owned(),
        "INSERT" => "Insert".to_owned(),
        "HOME" => "Home".to_owned(),
        "END" => "End".to_owned(),
        "PAGEUP" => "PageUp".to_owned(),
        "PAGEDOWN" => "PageDown".to_owned(),
        "UPARROW" => "UpArrow".to_owned(),
        "DOWNARROW" => "DownArrow".to_owned(),
        "LEFTARROW" => "LeftArrow".to_owned(),
        "RIGHTARROW" => "RightArrow".to_owned(),
        _ => key.to_owned(),
    }
}

fn nerd_font_icon_for_name(name: &str) -> Option<char> {
    let name = normalize_nerd_font_icon_name(name);
    if name.is_empty() {
        return None;
    }

    NERD_FONTS_CHAR_SELECT_CANDIDATES
        .iter()
        .find_map(|(character, candidate_name)| {
            (normalize_nerd_font_icon_name(candidate_name) == name).then_some(*character)
        })
}

fn normalize_nerd_font_icon_name(name: &str) -> String {
    let mut normalized = String::new();
    for ch in name.trim().chars() {
        match ch {
            '-' | ' ' => normalized.push('_'),
            _ => normalized.push(ch.to_ascii_lowercase()),
        }
    }

    while let Some(stripped) = normalized.strip_prefix("nf_") {
        normalized = stripped.to_owned();
    }
    normalized
}

fn default_command_palette_frecency_path() -> Option<PathBuf> {
    Some(default_rterm_state_dir()?.join("command-palette-frecency.json"))
}

fn default_char_select_recently_used_path() -> Option<PathBuf> {
    Some(default_rterm_state_dir()?.join("char-select-recently-used.json"))
}

fn default_rterm_state_dir() -> Option<PathBuf> {
    crate::platform::state_dir()
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
struct WindowCommandPaletteFrecency {
    uses: u64,
    last_used: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct WindowCommandPaletteFrecencyStore {
    sequence: u64,
    entries: BTreeMap<String, WindowCommandPaletteFrecency>,
}

#[derive(Debug, Default)]
struct WindowCommandPalette {
    query: String,
    selected: usize,
    augmented_entries: Vec<NativeCommandPaletteEntry>,
    launcher_args: Option<WindowShowLauncherArgs>,
    context_entries: Option<Vec<WindowCommandPaletteEntry>>,
    context_title: Option<String>,
    launcher_shortcut_prefix: String,
    launcher_fuzzy_filter: bool,
}

impl WindowCommandPalette {
    fn title(&self) -> &str {
        if let Some(title) = self.context_title.as_deref() {
            return title;
        }
        self.launcher_args
            .as_ref()
            .and_then(|args| args.title.as_deref())
            .unwrap_or(if self.launcher_args.is_some() {
                "Launcher"
            } else {
                "Command Palette"
            })
    }

    fn help_text(&self) -> Option<&str> {
        let args = self.launcher_args.as_ref()?;
        if args.flags.fuzzy || self.launcher_fuzzy_filter {
            Some(
                args.fuzzy_help_text
                    .as_deref()
                    .unwrap_or(DEFAULT_LAUNCHER_FUZZY_HELP_TEXT),
            )
        } else {
            Some(
                args.help_text
                    .as_deref()
                    .unwrap_or(DEFAULT_LAUNCHER_HELP_TEXT),
            )
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WindowSearch {
    query: String,
    current: Option<WindowSearchMatch>,
    match_type: WindowSearchMatchType,
    editing: bool,
}

impl Default for WindowSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            current: None,
            match_type: WindowSearchMatchType::default(),
            editing: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum WindowSearchMatchType {
    #[default]
    CaseSensitive,
    CaseInsensitive,
    Regex,
}

impl WindowSearchMatchType {
    const fn next(self) -> Self {
        match self {
            Self::CaseSensitive => Self::CaseInsensitive,
            Self::CaseInsensitive => Self::Regex,
            Self::Regex => Self::CaseSensitive,
        }
    }
}

const QUICK_SELECT_PATTERNS: &[WindowQuickSelectPattern] = &[
    WindowQuickSelectPattern::capture(r"\[[^]]*\]\(([^)]+)\)", 1),
    WindowQuickSelectPattern::whole(r"(?:https?://|git@|git://|ssh://|ftp://|file://)\S+"),
    WindowQuickSelectPattern::capture(r"--- a/(\S+)", 1),
    WindowQuickSelectPattern::capture(r"\+\+\+ b/(\S+)", 1),
    WindowQuickSelectPattern::capture(r"sha256:([0-9a-f]{64})", 1),
    WindowQuickSelectPattern::whole(r"(?:[.\w\-@~]+)?(?:/+[.\w\-@]+)+"),
    WindowQuickSelectPattern::whole(r"#[0-9a-fA-F]{6}"),
    WindowQuickSelectPattern::whole(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"),
    WindowQuickSelectPattern::whole(r"Qm[0-9a-zA-Z]{44}"),
    WindowQuickSelectPattern::whole(r"[0-9a-f]{7,40}"),
    WindowQuickSelectPattern::whole(r"\b(?:\d{1,3}\.){3}\d{1,3}\b"),
    WindowQuickSelectPattern::whole(r"[A-f0-9:]+:+[A-f0-9:]+[%\w\d]+"),
    WindowQuickSelectPattern::whole(r"0x[0-9a-fA-F]+"),
    WindowQuickSelectPattern::whole(r"[0-9]{4,}"),
    WindowQuickSelectPattern::whole(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"),
];

fn default_hyperlink_rules() -> Vec<NativeHyperlinkRule> {
    vec![
        NativeHyperlinkRule {
            regex: r"\((\w+://\S+)\)".to_owned(),
            format: "$1".to_owned(),
            highlight: 1,
        },
        NativeHyperlinkRule {
            regex: r"\[(\w+://\S+)\]".to_owned(),
            format: "$1".to_owned(),
            highlight: 1,
        },
        NativeHyperlinkRule {
            regex: r"\{(\w+://\S+)\}".to_owned(),
            format: "$1".to_owned(),
            highlight: 1,
        },
        NativeHyperlinkRule {
            regex: r"<(\w+://\S+)>".to_owned(),
            format: "$1".to_owned(),
            highlight: 1,
        },
        NativeHyperlinkRule {
            regex: r"\b\w+://\S+[)/a-zA-Z0-9-]+".to_owned(),
            format: "$0".to_owned(),
            highlight: 0,
        },
        NativeHyperlinkRule {
            regex: r"\b\w+@[\w-]+(\.[\w-]+)+\b".to_owned(),
            format: "mailto:$0".to_owned(),
            highlight: 0,
        },
    ]
}

#[derive(Clone, Copy)]
struct WindowQuickSelectPattern {
    regex: &'static str,
    capture: Option<usize>,
}

impl WindowQuickSelectPattern {
    const fn whole(regex: &'static str) -> Self {
        Self {
            regex,
            capture: None,
        }
    }

    const fn capture(regex: &'static str, capture: usize) -> Self {
        Self {
            regex,
            capture: Some(capture),
        }
    }
}

#[derive(Clone, Copy)]
struct WindowQuickSelectPatternRef<'a> {
    regex: &'a str,
    capture: Option<usize>,
}

impl<'a> WindowQuickSelectPatternRef<'a> {
    const fn whole(regex: &'a str) -> Self {
        Self {
            regex,
            capture: None,
        }
    }
}

fn quick_select_labels_for_alphabet_by_match(alphabet: &str, num_matches: usize) -> Vec<String> {
    let labels = quick_select_labels_for_alphabet(alphabet, num_matches);
    let mut labels_by_match = vec![String::new(); num_matches];
    for (match_index, label) in (0..num_matches).rev().zip(labels) {
        labels_by_match[match_index] = label;
    }
    labels_by_match
}

fn quick_select_labels_for_alphabet(alphabet: &str, num_matches: usize) -> Vec<String> {
    let alphabet = alphabet
        .chars()
        .map(|character| character.to_lowercase().to_string())
        .collect::<Vec<_>>();
    let mut primary = alphabet.clone();
    let mut secondary = Vec::new();

    while primary.len() + secondary.len() < num_matches {
        let Some(prefix) = primary.pop() else {
            break;
        };

        let remaining = num_matches - primary.len() - secondary.len();
        let prefixed = alphabet
            .iter()
            .take(remaining)
            .map(|character| format!("{prefix}{character}"))
            .collect::<Vec<_>>();
        secondary.splice(0..0, prefixed);
    }

    let secondary_len = secondary.len();
    primary
        .drain(..)
        .take(num_matches.saturating_sub(secondary_len))
        .chain(secondary)
        .collect()
}

fn find_window_quick_select_matches_with_patterns(
    terminal: &rterm_terminal::Terminal,
    patterns: &[WindowQuickSelectPatternRef<'_>],
    source_row_start: StableRowIndex,
    source_row_end: StableRowIndex,
) -> Vec<WindowSearchMatch> {
    let cells = terminal_search_cells(terminal)
        .into_iter()
        .filter(|cell| cell.source_row >= source_row_start && cell.source_row < source_row_end)
        .collect::<Vec<_>>();

    let mut matches = patterns
        .iter()
        .enumerate()
        .flat_map(|(pattern_index, pattern)| {
            quick_select_regex_window_search_matches(&cells, pattern_index, *pattern).into_iter()
        })
        .collect::<Vec<_>>();

    matches.sort_unstable_by_key(|candidate| {
        (
            candidate.full.source_row,
            candidate.full.start_column,
            candidate.pattern_index,
            candidate.selection.source_row,
            candidate.selection.start_column,
            std::cmp::Reverse(candidate.selection.end_source_row),
            std::cmp::Reverse(candidate.selection.end_column),
        )
    });
    let mut unique = Vec::new();
    let mut seen_text = HashSet::new();
    for candidate in matches {
        if unique
            .iter()
            .any(|kept| quick_select_matches_overlap(*kept, candidate.selection))
        {
            continue;
        }
        if !seen_text.insert(quick_select_match_text(&cells, candidate.selection)) {
            continue;
        }
        unique.push(candidate.selection);
    }
    unique
}

fn quick_select_match_text(cells: &[WindowSearchCell], selection: WindowSearchMatch) -> String {
    let mut text = String::new();
    let mut previous_source_row = None;
    for cell in cells.iter().filter(|cell| {
        cell.source_row >= selection.source_row
            && cell.source_row <= selection.end_source_row
            && (cell.source_row != selection.source_row || cell.column >= selection.start_column)
            && (cell.source_row != selection.end_source_row || cell.column <= selection.end_column)
    }) {
        if previous_source_row.is_some_and(|source_row| source_row != cell.source_row) {
            text.push('\n');
        }
        previous_source_row = Some(cell.source_row);
        text.push(cell.character);
    }
    text
}

fn quick_select_source_row_scope(
    terminal: &rterm_terminal::Terminal,
    scrollback_offset: usize,
    scope_lines: usize,
) -> (StableRowIndex, StableRowIndex) {
    let dimensions = terminal.stable_dimensions();
    let viewport_rows = usize::from(terminal.grid().size().rows);
    let viewport_top = dimensions
        .physical_top
        .saturating_sub(StableRowIndex::try_from(scrollback_offset).unwrap_or(StableRowIndex::MAX));
    let effective_scope_lines = scope_lines.max(viewport_rows);
    let effective_scope_lines =
        StableRowIndex::try_from(effective_scope_lines).unwrap_or(StableRowIndex::MAX);
    let viewport_rows = StableRowIndex::try_from(viewport_rows).unwrap_or(StableRowIndex::MAX);
    let retained = terminal.retained_stable_range();
    let row_start = viewport_top
        .saturating_sub(effective_scope_lines)
        .max(retained.start);
    let row_end = viewport_top
        .saturating_add(viewport_rows)
        .saturating_add(effective_scope_lines)
        .min(retained.end);
    (row_start, row_end)
}

#[derive(Clone, Copy)]
struct WindowQuickSelectCandidate {
    selection: WindowSearchMatch,
    full: WindowSearchMatch,
    pattern_index: usize,
}

fn quick_select_regex_window_search_matches(
    cells: &[WindowSearchCell],
    pattern_index: usize,
    pattern: WindowQuickSelectPatternRef<'_>,
) -> Vec<WindowQuickSelectCandidate> {
    let Ok(regex) = regex::Regex::new(pattern.regex) else {
        return Vec::new();
    };

    let mut text = String::new();
    let mut byte_to_cell_index = Vec::new();
    let mut previous_source_row = None;
    for (cell_index, cell) in cells.iter().enumerate() {
        if previous_source_row.is_some_and(|source_row| source_row != cell.source_row) {
            byte_to_cell_index.push(None);
            text.push('\n');
        }
        previous_source_row = Some(cell.source_row);

        for _ in 0..cell.character.len_utf8() {
            byte_to_cell_index.push(Some(cell_index));
        }
        text.push(cell.character);
    }

    let mut matches = Vec::new();
    match pattern.capture {
        Some(capture) => {
            for captures in regex.captures_iter(&text) {
                let Some(full) = captures.get(0) else {
                    continue;
                };
                let Some(selection) = captures.get(capture) else {
                    continue;
                };
                if selection.start() == selection.end() {
                    continue;
                }
                let Some(full) =
                    byte_range_to_window_search_match(&byte_to_cell_index, cells, full)
                else {
                    continue;
                };
                let Some(selection) =
                    byte_range_to_window_search_match(&byte_to_cell_index, cells, selection)
                else {
                    continue;
                };
                matches.push(WindowQuickSelectCandidate {
                    selection,
                    full,
                    pattern_index,
                });
            }
        }
        None => {
            for matched in regex.find_iter(&text) {
                if matched.start() == matched.end() {
                    continue;
                }
                let Some(selection) =
                    byte_range_to_window_search_match(&byte_to_cell_index, cells, matched)
                else {
                    continue;
                };
                matches.push(WindowQuickSelectCandidate {
                    selection,
                    full: selection,
                    pattern_index,
                });
            }
        }
    }
    matches
}

fn byte_range_to_window_search_match(
    byte_to_cell_index: &[Option<usize>],
    cells: &[WindowSearchCell],
    matched: regex::Match<'_>,
) -> Option<WindowSearchMatch> {
    let start_index = (*byte_to_cell_index.get(matched.start())?)?;
    let end_byte = matched.end().checked_sub(1)?;
    let end_index = (*byte_to_cell_index.get(end_byte)?)?;
    let start = cells.get(start_index)?;
    let end = cells.get(end_index)?;
    Some(WindowSearchMatch {
        domain: start.domain,
        source_row: start.source_row,
        start_column: start.column,
        end_source_row: end.source_row,
        end_column: end.column,
    })
}

fn quick_select_matches_overlap(left: WindowSearchMatch, right: WindowSearchMatch) -> bool {
    let left_start = (left.source_row, left.start_column);
    let left_end = (left.end_source_row, left.end_column);
    let right_start = (right.source_row, right.start_column);
    let right_end = (right.end_source_row, right.end_column);

    left_start <= right_end && right_start <= left_end
}

fn search_status(search: &WindowSearch) -> String {
    if search.query.is_empty() {
        "Search".to_owned()
    } else if search.current.is_some() {
        format!("Search: {}", search.query)
    } else {
        format!("Search: {} (no match)", search.query)
    }
}

fn notification_status(notification: &TerminalNotification) -> String {
    let title = notification
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty());
    let body = notification.body.trim();

    match (title, body.is_empty()) {
        (Some(title), false) => format!("Notification: {title} - {body}"),
        (Some(title), true) => format!("Notification: {title}"),
        (None, false) => format!("Notification: {body}"),
        (None, true) => "Notification".to_owned(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowSearchMatch {
    domain: TerminalScreenDomain,
    source_row: StableRowIndex,
    start_column: u16,
    end_source_row: StableRowIndex,
    end_column: u16,
}

impl WindowSearchMatch {
    fn is_retained(self, terminal: &Terminal) -> bool {
        self.domain == terminal.stable_dimensions().domain
            && self
                .end_source_row
                .checked_add(1)
                .is_some_and(|end| terminal.is_stable_range_fully_retained(self.source_row..end))
    }

    fn text_from_terminal(self, terminal: &Terminal) -> Option<String> {
        terminal.text_from_stable_selection(StableSelectionRange {
            start: StableSelectionCoordinate {
                domain: self.domain,
                row: self.source_row,
                column: usize::from(self.start_column),
            },
            end: StableSelectionCoordinate {
                domain: self.domain,
                row: self.end_source_row,
                column: usize::from(self.end_column),
            },
            rectangular: false,
        })
    }

    fn viewport_selection(self, terminal: &Terminal) -> Option<(usize, WindowSelection)> {
        let dimensions = terminal.stable_dimensions();
        if !self.is_retained(terminal) {
            return None;
        }
        let top = if self.source_row < dimensions.physical_top {
            self.source_row.max(dimensions.scrollback_top)
        } else {
            dimensions.physical_top
        };
        let offset = dimensions
            .physical_top
            .checked_sub(top)
            .and_then(|offset| usize::try_from(offset).ok())?;
        Some((
            offset,
            self.viewport_selection_for_top(dimensions.domain, top, terminal.grid().size())?,
        ))
    }

    fn viewport_selection_for_top(
        self,
        domain: TerminalScreenDomain,
        viewport_top: StableRowIndex,
        size: rterm_core::TerminalSize,
    ) -> Option<WindowSelection> {
        if size.rows == 0 || size.columns == 0 || self.domain != domain {
            return None;
        }

        let viewport_bottom = viewport_top
            .saturating_add(StableRowIndex::try_from(size.rows).unwrap_or(StableRowIndex::MAX));
        if self.end_source_row < viewport_top || self.source_row >= viewport_bottom {
            return None;
        }

        let last_row = size.rows.saturating_sub(1);
        let last_column = size.columns.saturating_sub(1);
        let start_row = if self.source_row < viewport_top {
            0
        } else {
            self.source_row
                .saturating_sub(viewport_top)
                .try_into()
                .unwrap_or(u16::MAX)
                .min(last_row)
        };
        let end_row = if self.end_source_row >= viewport_bottom {
            last_row
        } else {
            self.end_source_row
                .saturating_sub(viewport_top)
                .try_into()
                .unwrap_or(u16::MAX)
                .min(last_row)
        };
        let start_column = if self.source_row < viewport_top {
            0
        } else {
            self.start_column.min(last_column)
        };
        let end_column = if self.end_source_row >= viewport_bottom {
            last_column
        } else {
            self.end_column.min(last_column)
        };

        Some(WindowSelection::new(
            SelectionCell {
                row: start_row,
                column: start_column,
            },
            SelectionCell {
                row: end_row,
                column: end_column,
            },
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchDirection {
    Next,
    Previous,
}

fn find_window_search_match(
    matches: &[WindowSearchMatch],
    current: Option<WindowSearchMatch>,
    direction: SearchDirection,
) -> Option<WindowSearchMatch> {
    if matches.is_empty() {
        return None;
    }

    let Some(current) = current else {
        return match direction {
            SearchDirection::Next => matches.first().copied(),
            SearchDirection::Previous => matches.last().copied(),
        };
    };

    match direction {
        SearchDirection::Next => matches
            .iter()
            .copied()
            .find(|candidate| search_match_after(*candidate, current))
            .or_else(|| matches.first().copied()),
        SearchDirection::Previous => matches
            .iter()
            .rev()
            .copied()
            .find(|candidate| search_match_after(current, *candidate))
            .or_else(|| matches.last().copied()),
    }
}

fn find_window_search_page_match(
    matches: &[WindowSearchMatch],
    retained: std::ops::Range<StableRowIndex>,
    viewport_top: StableRowIndex,
    viewport_rows: usize,
    direction: SearchDirection,
) -> Option<WindowSearchMatch> {
    if matches.is_empty() || viewport_rows == 0 {
        return None;
    }

    let (page_start, page_end) = match direction {
        SearchDirection::Next => {
            let viewport_rows =
                StableRowIndex::try_from(viewport_rows).unwrap_or(StableRowIndex::MAX);
            let start = viewport_top.saturating_add(viewport_rows);
            (
                start.min(retained.end),
                start
                    .saturating_add(viewport_rows.saturating_sub(1))
                    .min(retained.end.saturating_sub(1)),
            )
        }
        SearchDirection::Previous => {
            let viewport_rows =
                StableRowIndex::try_from(viewport_rows).unwrap_or(StableRowIndex::MAX);
            let end = viewport_top.saturating_sub(1).max(retained.start);
            (
                viewport_top
                    .saturating_sub(viewport_rows)
                    .max(retained.start),
                end,
            )
        }
    };

    matches
        .iter()
        .copied()
        .find(|candidate| candidate.source_row >= page_start && candidate.source_row <= page_end)
}

fn search_match_after(candidate: WindowSearchMatch, current: WindowSearchMatch) -> bool {
    candidate.source_row > current.source_row
        || (candidate.source_row == current.source_row
            && candidate.start_column > current.start_column)
}

fn window_search_matches_with_type(
    terminal: &rterm_terminal::Terminal,
    query: &str,
    match_type: WindowSearchMatchType,
) -> Vec<WindowSearchMatch> {
    let Some(query) = WindowSearchQuery::parse(query, match_type) else {
        return Vec::new();
    };
    let cells = terminal_search_cells(terminal);

    match query {
        WindowSearchQuery::Literal(query) => literal_window_search_matches(&cells, &query, true),
        WindowSearchQuery::CaseInsensitiveLiteral(query) => {
            literal_window_search_matches(&cells, &query, false)
        }
        WindowSearchQuery::Regex(pattern) => regex_window_search_matches(&cells, pattern),
    }
}

enum WindowSearchQuery<'a> {
    Literal(Vec<char>),
    CaseInsensitiveLiteral(Vec<char>),
    Regex(&'a str),
}

impl<'a> WindowSearchQuery<'a> {
    fn parse(query: &'a str, match_type: WindowSearchMatchType) -> Option<Self> {
        if let Some(pattern) = strip_query_prefix_from_any(query, &["regex:"]) {
            return (!pattern.is_empty()).then_some(Self::Regex(pattern));
        }

        let (query, force_literal) = match strip_query_prefix_from_any(query, &["literal:"]) {
            Some(literal) => (literal, true),
            None => (query, false),
        };
        if match_type == WindowSearchMatchType::Regex && !force_literal {
            return (!query.is_empty()).then_some(Self::Regex(query));
        }

        let query: Vec<char> = query
            .chars()
            .filter(|character| !matches!(character, '\r' | '\n'))
            .collect();
        if query.is_empty() {
            return None;
        }

        Some(match match_type {
            WindowSearchMatchType::CaseSensitive | WindowSearchMatchType::Regex => {
                Self::Literal(query)
            }
            WindowSearchMatchType::CaseInsensitive => Self::CaseInsensitiveLiteral(query),
        })
    }
}

fn literal_window_search_matches(
    cells: &[WindowSearchCell],
    query: &[char],
    case_sensitive: bool,
) -> Vec<WindowSearchMatch> {
    let query: Vec<char> = query
        .iter()
        .copied()
        .filter(|character| !matches!(character, '\r' | '\n'))
        .collect();
    if query.is_empty() {
        return Vec::new();
    }

    if cells.len() < query.len() {
        return Vec::new();
    }

    cells
        .windows(query.len())
        .filter_map(|candidate| {
            if candidate
                .iter()
                .zip(query.iter().copied())
                .all(|(cell, query_character)| {
                    if case_sensitive {
                        cell.character == query_character
                    } else {
                        cell.character.eq_ignore_ascii_case(&query_character)
                    }
                })
            {
                let start = candidate.first()?;
                let end = candidate.last()?;
                Some(WindowSearchMatch {
                    domain: start.domain,
                    source_row: start.source_row,
                    start_column: start.column,
                    end_source_row: end.source_row,
                    end_column: end.end_column,
                })
            } else {
                None
            }
        })
        .collect()
}

fn regex_window_search_matches(
    cells: &[WindowSearchCell],
    pattern: &str,
) -> Vec<WindowSearchMatch> {
    let Ok(pattern) = regex::Regex::new(pattern) else {
        return Vec::new();
    };

    let mut text = String::new();
    let mut byte_to_cell_index = Vec::new();
    for (cell_index, cell) in cells.iter().enumerate() {
        for _ in 0..cell.character.len_utf8() {
            byte_to_cell_index.push(cell_index);
        }
        text.push(cell.character);
    }

    pattern
        .find_iter(&text)
        .filter_map(|matched| {
            if matched.start() == matched.end() {
                return None;
            }

            let start_index = *byte_to_cell_index.get(matched.start())?;
            let end_byte = matched.end().checked_sub(1)?;
            let end_index = *byte_to_cell_index.get(end_byte)?;
            let start = cells.get(start_index)?;
            let end = cells.get(end_index)?;
            Some(WindowSearchMatch {
                domain: start.domain,
                source_row: start.source_row,
                start_column: start.column,
                end_source_row: end.source_row,
                end_column: end.end_column,
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
struct WindowSearchCell {
    character: char,
    domain: TerminalScreenDomain,
    source_row: StableRowIndex,
    column: u16,
    end_column: u16,
}

fn terminal_search_cells(terminal: &rterm_terminal::Terminal) -> Vec<WindowSearchCell> {
    let dimensions = terminal.stable_dimensions();
    let size = terminal.grid().size();
    let mut result = Vec::new();
    let mut append_row = |source_row: StableRowIndex, cells: &[rterm_terminal::Cell]| {
        let mut row = Vec::new();
        for (column, cell) in cells.iter().take(usize::from(size.columns)).enumerate() {
            if cell.is_continuation() {
                continue;
            }
            let Ok(column) = u16::try_from(column) else {
                continue;
            };
            row.extend(cell.text().chars().map(|character| WindowSearchCell {
                character,
                domain: dimensions.domain,
                source_row,
                column,
                end_column: column.saturating_add(u16::from(cell.columns()).saturating_sub(1)),
            }));
        }
        while row.last().is_some_and(|cell| cell.character == ' ') {
            row.pop();
        }
        result.extend(row);
    };

    if dimensions.domain == TerminalScreenDomain::Main {
        for (index, line) in terminal.scrollback().iter().enumerate() {
            append_row(
                dimensions
                    .scrollback_top
                    .saturating_add(StableRowIndex::try_from(index).unwrap_or(StableRowIndex::MAX)),
                line.cells(),
            );
        }
    }
    for row in 0..size.rows {
        let cells = (0..size.columns)
            .filter_map(|column| terminal.grid().get(row, column).cloned())
            .collect::<Vec<_>>();
        append_row(
            dimensions
                .physical_top
                .saturating_add(StableRowIndex::try_from(row).unwrap_or(StableRowIndex::MAX)),
            &cells,
        );
    }
    result
}

#[derive(Clone, Copy)]
struct WindowMouseEvent {
    kind: WindowMouseEventKind,
    column: u16,
    row: u16,
    modifiers: ModifiersState,
}

#[derive(Clone, Copy)]
enum WindowMouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

fn iterm_mouse_info_for_event(
    mouse_cell: PaneMouseCell,
    source_row: usize,
    kind: WindowMouseEventKind,
    modifiers: ModifiersState,
    mut side_effects: u16,
) -> Option<ItermMouseInfo> {
    let (button, click_count, event_type) = match kind {
        WindowMouseEventKind::Down(button) => (iterm_mouse_button_number(button)?, 1, 1),
        WindowMouseEventKind::Up(button) => (iterm_mouse_button_number(button)?, 1, 0),
        WindowMouseEventKind::Drag(button) => {
            side_effects |= ITERM_MOUSE_DRAG_SIDE_EFFECT;
            (iterm_mouse_button_number(button)?, 0, 2)
        }
        WindowMouseEventKind::Moved => (0, 0, 4),
        WindowMouseEventKind::ScrollUp
        | WindowMouseEventKind::ScrollDown
        | WindowMouseEventKind::ScrollLeft
        | WindowMouseEventKind::ScrollRight => return None,
    };

    Some(ItermMouseInfo {
        pane_id: mouse_cell.pane_id,
        x: mouse_cell.column,
        y: source_row,
        button,
        click_count,
        modifier_mask: iterm_mouse_modifier_mask(modifiers),
        side_effects,
        event_type,
    })
}

fn iterm_mouse_modifier_mask(modifiers: ModifiersState) -> u8 {
    let mut mask = 0;
    if modifiers.control_key() {
        mask |= ITERM_MOUSE_CONTROL_MODIFIER;
    }
    if modifiers.alt_key() {
        mask |= ITERM_MOUSE_OPTION_MODIFIER;
    }
    if modifiers.super_key() {
        mask |= ITERM_MOUSE_COMMAND_MODIFIER;
    }
    if modifiers.shift_key() {
        mask |= ITERM_MOUSE_SHIFT_MODIFIER;
    }
    mask
}

const fn iterm_mouse_button_number(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        _ => None,
    }
}

fn encode_window_mouse_event(event: WindowMouseEvent, mode: MouseInputMode) -> Option<Vec<u8>> {
    encode_window_mouse_event_with_optional_pixels(event, None, mode)
}

fn encode_window_mouse_event_with_pixels(
    event: WindowMouseEvent,
    x_pixels: u16,
    y_pixels: u16,
    mode: MouseInputMode,
) -> Option<Vec<u8>> {
    encode_window_mouse_event_with_optional_pixels(event, Some((x_pixels, y_pixels)), mode)
}

fn encode_window_mouse_event_with_optional_pixels(
    event: WindowMouseEvent,
    pixels: Option<(u16, u16)>,
    mode: MouseInputMode,
) -> Option<Vec<u8>> {
    if !window_mouse_reporting_allows(event.kind, mode.reporting()) {
        return None;
    }

    let mut code = match event.kind {
        WindowMouseEventKind::Down(button) | WindowMouseEventKind::Up(button) => {
            window_mouse_button_code(button)?
        }
        WindowMouseEventKind::Drag(button) => window_mouse_button_code(button)? + 32,
        WindowMouseEventKind::Moved => 35,
        WindowMouseEventKind::ScrollUp => 64,
        WindowMouseEventKind::ScrollDown => 65,
        WindowMouseEventKind::ScrollLeft => 66,
        WindowMouseEventKind::ScrollRight => 67,
    };

    if event.modifiers.shift_key() {
        code += 4;
    }
    if event.modifiers.alt_key() {
        code += 8;
    }
    if event.modifiers.control_key() {
        code += 16;
    }

    let column = event.column.checked_add(1)?;
    let row = event.row.checked_add(1)?;

    match mode.protocol() {
        MouseProtocolMode::Sgr => {
            let final_byte = if matches!(event.kind, WindowMouseEventKind::Up(_)) {
                b'm'
            } else {
                b'M'
            };
            Some(format!("\x1b[<{code};{column};{row}{}", final_byte as char).into_bytes())
        }
        MouseProtocolMode::SgrPixels => {
            let (x_pixels, y_pixels) = pixels.unwrap_or((column, row));
            let final_byte = if matches!(event.kind, WindowMouseEventKind::Up(_)) {
                b'm'
            } else {
                b'M'
            };
            Some(format!("\x1b[<{code};{x_pixels};{y_pixels}{}", final_byte as char).into_bytes())
        }
        MouseProtocolMode::Utf8 => encode_utf8_window_mouse_event(event.kind, code, column, row),
        MouseProtocolMode::Urxvt => encode_urxvt_window_mouse_event(event.kind, code, column, row),
        MouseProtocolMode::X10 => encode_legacy_window_mouse_event(event.kind, code, column, row),
    }
}

fn mouse_report_pixel_coordinate(value: f64) -> Option<u16> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }

    let value = value.floor();
    if value > f64::from(u16::MAX.saturating_sub(1)) {
        return None;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixel = value as u16;
    pixel.checked_add(1)
}

fn window_mouse_reporting_allows(
    kind: WindowMouseEventKind,
    reporting: MouseReportingMode,
) -> bool {
    match reporting {
        MouseReportingMode::None => false,
        MouseReportingMode::Normal => matches!(
            kind,
            WindowMouseEventKind::Down(_)
                | WindowMouseEventKind::Up(_)
                | WindowMouseEventKind::ScrollUp
                | WindowMouseEventKind::ScrollDown
                | WindowMouseEventKind::ScrollLeft
                | WindowMouseEventKind::ScrollRight
        ),
        MouseReportingMode::ButtonEvent => !matches!(kind, WindowMouseEventKind::Moved),
        MouseReportingMode::AnyEvent => true,
    }
}

fn encode_legacy_window_mouse_event(
    kind: WindowMouseEventKind,
    mut code: u16,
    column: u16,
    row: u16,
) -> Option<Vec<u8>> {
    if matches!(kind, WindowMouseEventKind::Up(_)) {
        code = legacy_window_mouse_release_code(code);
    }

    Some(vec![
        0x1b,
        b'[',
        b'M',
        legacy_mouse_byte(code)?,
        legacy_mouse_byte(column)?,
        legacy_mouse_byte(row)?,
    ])
}

fn encode_utf8_window_mouse_event(
    kind: WindowMouseEventKind,
    mut code: u16,
    column: u16,
    row: u16,
) -> Option<Vec<u8>> {
    if matches!(kind, WindowMouseEventKind::Up(_)) {
        code = legacy_window_mouse_release_code(code);
    }

    let mut bytes = b"\x1b[M".to_vec();
    push_utf8_mouse_value(&mut bytes, code)?;
    push_utf8_mouse_value(&mut bytes, column)?;
    push_utf8_mouse_value(&mut bytes, row)?;
    Some(bytes)
}

fn encode_urxvt_window_mouse_event(
    kind: WindowMouseEventKind,
    mut code: u16,
    column: u16,
    row: u16,
) -> Option<Vec<u8>> {
    if matches!(kind, WindowMouseEventKind::Up(_)) {
        code = legacy_window_mouse_release_code(code);
    }

    let encoded_code = code.checked_add(32)?;
    Some(format!("\x1b[{encoded_code};{column};{row}M").into_bytes())
}

fn legacy_mouse_byte(value: u16) -> Option<u8> {
    u8::try_from(value.checked_add(32)?).ok()
}

fn push_utf8_mouse_value(bytes: &mut Vec<u8>, value: u16) -> Option<()> {
    let ch = char::from_u32(u32::from(value.checked_add(32)?))?;
    let mut buffer = [0; 4];
    bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
    Some(())
}

const fn legacy_window_mouse_release_code(code: u16) -> u16 {
    3 + (code & !0b11)
}

const fn window_mouse_button_code(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
        _ => None,
    }
}

fn window_mouse_wheel_kind(delta: MouseScrollDelta) -> Option<WindowMouseEventKind> {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => wheel_kind_from_axes(f64::from(x), f64::from(y)),
        MouseScrollDelta::PixelDelta(position) => wheel_kind_from_axes(position.x, position.y),
    }
}

fn native_mouse_assignment_wheel_button_from_delta(
    delta: MouseScrollDelta,
) -> Option<NativeMouseAssignmentButton> {
    match window_mouse_wheel_kind(delta)? {
        WindowMouseEventKind::ScrollUp => Some(NativeMouseAssignmentButton::WheelUp),
        WindowMouseEventKind::ScrollDown => Some(NativeMouseAssignmentButton::WheelDown),
        _ => None,
    }
}

fn wheel_kind_from_axes(x: f64, y: f64) -> Option<WindowMouseEventKind> {
    if y > 0.0 {
        Some(WindowMouseEventKind::ScrollUp)
    } else if y < 0.0 {
        Some(WindowMouseEventKind::ScrollDown)
    } else if x > 0.0 {
        Some(WindowMouseEventKind::ScrollRight)
    } else if x < 0.0 {
        Some(WindowMouseEventKind::ScrollLeft)
    } else {
        None
    }
}

fn pixel_axis_to_cell(value: f64, cell_size: u32) -> Option<u16> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }

    let cell = (value / f64::from(cell_size)).floor();
    if cell > f64::from(u16::MAX) {
        return None;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Some(cell as u16)
}

fn search_match_source_selection(matched: WindowSearchMatch) -> WindowSourceSelection {
    WindowSourceSelection::new(
        SelectionSourceCell {
            domain: matched.domain,
            row: matched.source_row,
            column: usize::from(matched.start_column),
        },
        SelectionSourceCell {
            domain: matched.domain,
            row: matched.end_source_row,
            column: usize::from(matched.end_column),
        },
    )
}

fn pane_copy_overlay_rendering(ui: &PaneUiState) -> bool {
    ui.copy_search_mode() == Some(WindowCopySearchMode::Copy)
        || ui
            .retained_copy_mode()
            .is_some_and(|copy_mode| copy_mode.search_direction.is_some())
}

fn pane_overlay_source_selection(
    terminal: &Terminal,
    ui: &PaneUiState,
    word_boundary: &str,
) -> Option<WindowSourceSelection> {
    if let Some(quick_select) = ui.quick_select() {
        return quick_select
            .current_match()
            .map(search_match_source_selection);
    }

    match ui.copy_search_mode() {
        Some(WindowCopySearchMode::Search) => ui
            .search()
            .and_then(|search| search.current)
            .map(search_match_source_selection),
        Some(WindowCopySearchMode::Copy) => ui
            .copy_mode()
            .and_then(|copy_mode| copy_mode_source_selection(copy_mode, terminal, word_boundary))
            .or_else(|| {
                ui.retained_search()
                    .and_then(|search| search.current)
                    .map(search_match_source_selection)
            }),
        None => ui
            .ordinary_selection
            .map(StableOrdinarySelection::source_selection),
    }
}

fn pane_viewport_top(terminal: &Terminal, ui: &PaneUiState) -> StableRowIndex {
    ui.stable_viewport
        .active_top(terminal)
        .unwrap_or(terminal.stable_dimensions().physical_top)
}

fn pane_overlay_viewport_selection(
    terminal: &Terminal,
    ui: &PaneUiState,
    word_boundary: &str,
) -> Option<WindowSelection> {
    pane_overlay_source_selection(terminal, ui, word_boundary).and_then(|selection| {
        selection.viewport_selection(
            terminal.stable_dimensions().domain,
            pane_viewport_top(terminal, ui),
            terminal.grid().size(),
        )
    })
}

fn pane_inactive_search_selections(terminal: &Terminal, ui: &PaneUiState) -> Vec<WindowSelection> {
    let size = terminal.grid().size();
    if !pane_copy_overlay_rendering(ui) || size.rows == 0 || size.columns == 0 {
        return Vec::new();
    }
    let Some(search) = ui.retained_search() else {
        return Vec::new();
    };
    if search.query.is_empty() {
        return Vec::new();
    }

    let dimensions = terminal.stable_dimensions();
    let viewport_top = pane_viewport_top(terminal, ui);
    let Some(matches) = ui.cached_search_matches(terminal) else {
        return Vec::new();
    };
    matches
        .iter()
        .copied()
        .filter(|matched| Some(*matched) != search.current)
        .filter_map(|matched| {
            matched.viewport_selection_for_top(dimensions.domain, viewport_top, size)
        })
        .collect()
}

fn quick_select_cells_for_pane(
    terminal: &Terminal,
    viewport: PaneStableViewport,
    quick_select: &WindowQuickSelect,
    rect: PaneRenderRect,
    palette: &NativeResolvedPalette,
) -> Vec<RenderCell> {
    if quick_select.matches.is_empty() || rect.rows == 0 || rect.columns == 0 {
        return Vec::new();
    }

    let size = terminal.grid().size();
    if size.rows == 0 || size.columns == 0 {
        return Vec::new();
    }
    let dimensions = terminal.stable_dimensions();
    let viewport_top = viewport
        .active_top(terminal)
        .unwrap_or(dimensions.physical_top);
    let viewport_bottom = viewport_top
        .saturating_add(StableRowIndex::try_from(size.rows).unwrap_or(StableRowIndex::MAX));
    let foreground = palette
        .quick_select_label_fg
        .map_or(DEFAULT_QUICK_SELECT_LABEL_FG_COLOR, native_color_spec_to_render_color);
    let background = palette
        .quick_select_label_bg
        .map_or(DEFAULT_QUICK_SELECT_LABEL_BG_COLOR, native_color_spec_to_render_color);
    let input_prefix = quick_select.input.to_ascii_lowercase();
    let mut cells = Vec::new();

    for (matched, label) in quick_select.matches.iter().zip(&quick_select.labels) {
        if matched.domain != dimensions.domain
            || label.is_empty()
            || matched.source_row < viewport_top
            || matched.source_row >= viewport_bottom
            || (!input_prefix.is_empty() && !label.starts_with(&input_prefix))
        {
            continue;
        }
        let row = matched
            .source_row
            .saturating_sub(viewport_top)
            .try_into()
            .unwrap_or(u16::MAX);
        if row >= size.rows || row >= rect.rows {
            continue;
        }

        let mut column = matched.start_column;
        for ch in label.chars() {
            let width = terminal.char_display_width(ch);
            if width == 0 {
                continue;
            }
            let Some(end_column) = column.checked_add(width) else {
                break;
            };
            if column >= size.columns
                || column >= rect.columns
                || end_column > size.columns
                || end_column > rect.columns
            {
                break;
            }
            cells.push(ui_render_cell(
                row, column, ch, foreground, background, true,
            ));
            for continuation_column in column.saturating_add(1)..end_column {
                cells.push(ui_render_cell(
                    row,
                    continuation_column,
                    ' ',
                    foreground,
                    background,
                    true,
                ));
            }
            column = end_column;
        }
    }

    cells
}

#[expect(
    clippy::too_many_lines,
    reason = "compatibility reducer remains linear to preserve evaluation and precedence order"
)]
#[allow(clippy::too_many_arguments)]
fn pane_presentation_snapshot(
    base: TerminalRenderSnapshot,
    terminal: &Terminal,
    ui: &PaneUiState,
    rect: PaneRenderRect,
    palette: &NativeResolvedPalette,
    word_boundary: &str,
    suppress_pane_overlay: bool,
    quick_select_remove_styling: bool,
    foreground_text_hsb: NativeInactivePaneHsb,
    text_background_opacity: NativeTextBackgroundOpacity,
    window_background_opacity: NativeTextBackgroundOpacity,
    inactive_pane_hsb: Option<NativeInactivePaneHsb>,
    text_min_contrast_ratio: Option<NativeTextMinContrastRatio>,
    bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors,
) -> TerminalRenderSnapshot {
    let size = terminal.grid().size();
    let mut snapshot = base;
    if !suppress_pane_overlay && ui.quick_select().is_some() && quick_select_remove_styling {
        snapshot = quick_select_remove_styling_snapshot(snapshot);
    }

    let inactive_search_selections = if suppress_pane_overlay {
        Vec::new()
    } else {
        pane_inactive_search_selections(terminal, ui)
    };
    if !inactive_search_selections.is_empty() {
        let foreground = palette
            .copy_mode_inactive_highlight_fg
            .map(native_color_spec_to_render_color)
            .map(Some);
        let background = palette
            .copy_mode_inactive_highlight_bg
            .map(native_color_spec_to_render_color);
        snapshot = snapshot.with_selection_colors_overlay(
            |row, column| {
                inactive_search_selections
                    .iter()
                    .any(|selection| selection.contains(row, column, size))
            },
            foreground,
            background,
        );
    }

    let selection = if suppress_pane_overlay {
        ui.ordinary_selection
            .map(StableOrdinarySelection::source_selection)
            .and_then(|selection| {
                selection.viewport_selection(
                    terminal.stable_dimensions().domain,
                    pane_viewport_top(terminal, ui),
                    terminal.grid().size(),
                )
            })
    } else {
        pane_overlay_viewport_selection(terminal, ui, word_boundary)
    };
    if let Some(selection) = selection {
        let copy_overlay_rendering = !suppress_pane_overlay && pane_copy_overlay_rendering(ui);
        let foreground = if copy_overlay_rendering {
            palette
                .copy_mode_active_highlight_fg
                .map(native_color_spec_to_render_color)
                .map(Some)
                .or(palette.selection_fg)
        } else if !suppress_pane_overlay && ui.quick_select().is_some() {
            palette
                .quick_select_match_fg
                .map(native_color_spec_to_render_color)
                .map(Some)
                .or(palette.selection_fg)
        } else {
            palette.selection_fg
        };
        let background = if copy_overlay_rendering {
            palette
                .copy_mode_active_highlight_bg
                .map(native_color_spec_to_render_color)
                .or(palette.selection_bg)
        } else if !suppress_pane_overlay && ui.quick_select().is_some() {
            palette
                .quick_select_match_bg
                .map(native_color_spec_to_render_color)
                .or(palette.selection_bg)
        } else {
            palette.selection_bg
        };
        snapshot = snapshot.with_selection_colors_overlay(
            |row, column| selection.contains(row, column, size),
            foreground,
            background,
        );
    }

    if !suppress_pane_overlay && let Some(quick_select) = ui.quick_select() {
        snapshot = snapshot.with_overlay_cells(quick_select_cells_for_pane(
            terminal,
            ui.stable_viewport,
            quick_select,
            rect,
            palette,
        ));
    }

    snapshot = foreground_text_hsb_snapshot(snapshot, foreground_text_hsb);
    snapshot = text_background_opacity_snapshot(snapshot, text_background_opacity);
    snapshot = window_background_opacity_snapshot(
        snapshot,
        window_background_opacity,
        palette.background,
    );
    if let Some(hsb) = inactive_pane_hsb {
        snapshot = inactive_pane_snapshot(snapshot, hsb, palette.foreground, palette.background);
    }

    let ansi = std::array::from_fn(|index| {
        if index < palette.ansi.len() {
            palette.ansi[index]
        } else {
            palette.brights[index - palette.ansi.len()]
        }
    });
    text_min_contrast_snapshot(
        snapshot,
        text_min_contrast_ratio,
        color_to_rgba(palette.foreground, DEFAULT_RENDER_FOREGROUND_RGBA),
        color_to_rgba(palette.background, DEFAULT_RENDER_BACKGROUND_RGBA),
        bold_brightens_ansi_colors,
        Some(ansi),
        Some(palette.indexed),
    )
}

fn inactive_pane_snapshot(
    snapshot: TerminalRenderSnapshot,
    hsb: NativeInactivePaneHsb,
    foreground: Color,
    background: Color,
) -> TerminalRenderSnapshot {
    snapshot.with_cell_colors_mapped(|role, color| {
        inactive_pane_color(role, color, hsb, foreground, background)
    })
}

fn foreground_text_hsb_snapshot(
    snapshot: TerminalRenderSnapshot,
    hsb: NativeInactivePaneHsb,
) -> TerminalRenderSnapshot {
    if hsb == DEFAULT_FOREGROUND_TEXT_HSB {
        return snapshot;
    }

    snapshot.with_cell_colors_mapped(|role, color| foreground_text_hsb_color(role, color, hsb))
}

fn quick_select_remove_styling_snapshot(
    snapshot: TerminalRenderSnapshot,
) -> TerminalRenderSnapshot {
    snapshot.with_cells_mapped(|mut cell| {
        cell.foreground = Color::Default;
        cell.background = Color::Default;
        cell.underline_color = Color::Default;
        cell.underline_style = UnderlineStyle::None;
        cell.bold = false;
        cell.faint = false;
        cell.italic = false;
        cell.blink = false;
        cell.rapid_blink = false;
        cell.underline = false;
        cell.double_underline = false;
        cell.conceal = false;
        cell.strikethrough = false;
        cell.overline = false;
        cell.vertical_align = VerticalAlign::default();
        cell.inverse = false;
        cell.hyperlink = None;
        cell
    })
}

struct CompiledHyperlinkRules {
    rules: Vec<CompiledHyperlinkRule>,
}

struct CompiledHyperlinkRule {
    // Keep invalid patterns as None so they retain the previous behavior of
    // being skipped without making the renderer retry compilation per row.
    regex: Option<regex::Regex>,
    format: String,
    highlight: usize,
}

impl CompiledHyperlinkRules {
    fn from_rules(rules: &[NativeHyperlinkRule]) -> Self {
        Self {
            rules: rules
                .iter()
                .map(|rule| CompiledHyperlinkRule {
                    regex: regex::Regex::new(&rule.regex).ok(),
                    format: rule.format.clone(),
                    highlight: rule.highlight,
                })
                .collect(),
        }
    }
}

impl NativeWindowApp {
    fn compiled_hyperlink_rules(&self) -> Ref<'_, CompiledHyperlinkRules> {
        if self.hyperlink_rule_cache.borrow().is_none() {
            let compiled = CompiledHyperlinkRules::from_rules(&self.hyperlink_rules);
            self.hyperlink_rule_cache.borrow_mut().replace(compiled);
        }

        Ref::map(self.hyperlink_rule_cache.borrow(), |cache| {
            cache
                .as_ref()
                .expect("hyperlink rule cache populated before borrowing")
        })
    }

    fn invalidate_hyperlink_rule_cache(&self) {
        self.hyperlink_rule_cache.borrow_mut().take();
    }

    fn hyperlink_rules_snapshot(&self, snapshot: TerminalRenderSnapshot) -> TerminalRenderSnapshot {
        if self.hyperlink_rules.is_empty() {
            return snapshot;
        }

        let compiled = self.compiled_hyperlink_rules();
        hyperlink_rules_snapshot_with_compiled(snapshot, &compiled.rules)
    }

    fn hyperlink_rule_at_cell(
        &self,
        snapshot: &TerminalRenderSnapshot,
        row: u16,
        column: u16,
    ) -> Option<Arc<str>> {
        if self.hyperlink_rules.is_empty() {
            return None;
        }

        let compiled = self.compiled_hyperlink_rules();
        hyperlink_rule_cell_links(snapshot.iter_cells(), &compiled.rules).remove(&(row, column))
    }
}

fn hyperlink_rules_snapshot_with_compiled(
    snapshot: TerminalRenderSnapshot,
    rules: &[CompiledHyperlinkRule],
) -> TerminalRenderSnapshot {
    let hyperlinks = hyperlink_rule_cell_links(snapshot.iter_cells(), rules);
    if hyperlinks.is_empty() {
        return snapshot;
    }

    snapshot.with_cells_mapped(|mut cell| {
        if cell.hyperlink.is_none()
            && let Some(hyperlink) = hyperlinks.get(&(cell.row, cell.column))
        {
            cell.hyperlink = Some(hyperlink.clone());
        }
        cell
    })
}

fn hyperlink_rule_cell_links<'a>(
    cells: impl Iterator<Item = &'a RenderCell>,
    rules: &[CompiledHyperlinkRule],
) -> HashMap<(u16, u16), Arc<str>> {
    if rules.is_empty() {
        return HashMap::new();
    }

    let mut cells_by_row: BTreeMap<u16, Vec<&RenderCell>> = BTreeMap::new();
    for cell in cells {
        cells_by_row.entry(cell.row).or_default().push(cell);
    }

    let mut links = HashMap::new();
    for row_cells in cells_by_row.values_mut() {
        row_cells.sort_by_key(|cell| cell.column);
        apply_hyperlink_rules_to_row(row_cells, rules, &mut links);
    }

    links
}

fn apply_hyperlink_rules_to_row(
    row_cells: &[&RenderCell],
    rules: &[CompiledHyperlinkRule],
    links: &mut HashMap<(u16, u16), Arc<str>>,
) {
    let Some(row) = row_cells.first().map(|cell| cell.row) else {
        return;
    };
    let mut text = String::new();
    let mut byte_to_cell = Vec::new();
    let mut next_column = 0u16;
    let mut existing_hyperlinks = HashSet::new();

    for cell in row_cells {
        while next_column < cell.column {
            byte_to_cell.push(None);
            text.push(' ');
            next_column = next_column.saturating_add(1);
        }
        if cell.continuation {
            next_column = next_column.max(cell.column.saturating_add(1));
            continue;
        }
        for _ in 0..cell.text.len() {
            byte_to_cell.push(Some((row, cell.column)));
        }
        text.push_str(&cell.text);
        if cell.hyperlink.is_some() {
            existing_hyperlinks.insert((row, cell.column));
        }
        next_column = cell.column.saturating_add(u16::from(cell.columns).max(1));
    }

    for rule in rules {
        let Some(regex) = rule.regex.as_ref() else {
            continue;
        };
        for captures in regex.captures_iter(&text) {
            let Some(full_match) = captures.get(0) else {
                continue;
            };
            if full_match.start() == full_match.end() {
                continue;
            }
            let Some(highlight) = captures.get(rule.highlight).or_else(|| captures.get(0)) else {
                continue;
            };
            if highlight.start() == highlight.end() {
                continue;
            }
            let hyperlink = hyperlink_rule_format_uri(&rule.format, &captures);
            if hyperlink.is_empty() || !hyperlink.contains(':') {
                continue;
            }
            let hyperlink = Arc::<str>::from(hyperlink);
            for byte_index in highlight.start()..highlight.end() {
                let Some(Some(position)) = byte_to_cell.get(byte_index).copied() else {
                    continue;
                };
                if existing_hyperlinks.contains(&position) {
                    continue;
                }
                links.entry(position).or_insert_with(|| hyperlink.clone());
            }
        }
    }
}

fn hyperlink_rule_format_uri(format: &str, captures: &regex::Captures<'_>) -> String {
    let mut uri = String::new();
    let mut chars = format.char_indices().peekable();

    while let Some((_, character)) = chars.next() {
        if character != '$' {
            uri.push(character);
            continue;
        }

        let mut capture = String::new();
        while let Some((_, next)) = chars.peek().copied() {
            if !next.is_ascii_digit() {
                break;
            }
            capture.push(next);
            chars.next();
        }
        if capture.is_empty() {
            uri.push('$');
            continue;
        }
        if let Ok(index) = capture.parse::<usize>()
            && let Some(matched) = captures.get(index)
        {
            uri.push_str(matched.as_str());
        }
    }

    uri
}

fn text_background_opacity_snapshot(
    snapshot: TerminalRenderSnapshot,
    opacity: NativeTextBackgroundOpacity,
) -> TerminalRenderSnapshot {
    if opacity == DEFAULT_TEXT_BACKGROUND_OPACITY {
        return snapshot;
    }

    snapshot
        .with_cell_colors_mapped(|role, color| text_background_opacity_color(role, color, opacity))
}

fn window_background_opacity_snapshot(
    snapshot: TerminalRenderSnapshot,
    opacity: NativeTextBackgroundOpacity,
    background: Color,
) -> TerminalRenderSnapshot {
    if opacity == DEFAULT_WINDOW_BACKGROUND_OPACITY {
        return snapshot;
    }

    snapshot.with_cell_colors_mapped(|role, color| {
        window_background_opacity_color(role, color, opacity, background)
    })
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "ownership transfer is intentional for compatibility palette updates"
)]
fn text_min_contrast_snapshot(
    snapshot: TerminalRenderSnapshot,
    ratio: Option<NativeTextMinContrastRatio>,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors,
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
) -> TerminalRenderSnapshot {
    let Some(ratio) = ratio else {
        return snapshot;
    };
    let min_ratio = ratio.as_f64();
    if min_ratio <= 0.0 {
        return snapshot;
    }

    snapshot.with_cells_mapped(|cell| {
        text_min_contrast_cell(
            cell,
            min_ratio,
            default_foreground,
            default_background,
            bold_brightens_ansi_colors,
            ansi_palette,
            indexed_palette,
        )
    })
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "ownership transfer is intentional for compatibility palette updates"
)]
fn text_min_contrast_cell(
    mut cell: RenderCell,
    min_ratio: f64,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors,
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
) -> RenderCell {
    if cell.ch == ' ' || cell.conceal || cell.faint {
        return cell;
    }

    let (foreground, background) = text_effective_cell_colors(
        &cell,
        default_foreground,
        default_background,
        bold_brightens_ansi_colors,
        ansi_palette,
        indexed_palette,
    );
    let Some(adjusted) = text_contrast_adjusted_foreground(foreground, background, min_ratio)
    else {
        return cell;
    };
    let adjusted = rgba_to_color(adjusted);

    if cell.inverse {
        cell.background = adjusted;
    } else {
        cell.foreground = adjusted;
    }

    cell
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "ownership transfer is intentional for compatibility palette updates"
)]
fn text_effective_cell_colors(
    cell: &RenderCell,
    default_foreground: [u8; 4],
    default_background: [u8; 4],
    bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors,
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
) -> ([u8; 4], [u8; 4]) {
    let foreground_color = text_effective_cell_foreground(cell, bold_brightens_ansi_colors);
    let foreground = native_color_to_rgba(
        foreground_color,
        default_foreground,
        ansi_palette,
        indexed_palette,
    );
    let background = native_color_to_rgba(
        cell.background,
        default_background,
        ansi_palette,
        indexed_palette,
    );

    if cell.inverse {
        (background, foreground)
    } else {
        (foreground, background)
    }
}

fn text_effective_cell_foreground(
    cell: &RenderCell,
    bold_brightens_ansi_colors: NativeBoldBrightensAnsiColors,
) -> Color {
    let Color::Indexed(index @ 0..=7) = cell.foreground else {
        return cell.foreground;
    };

    if cell.bold && bold_brightens_ansi_colors != NativeBoldBrightensAnsiColors::No {
        Color::Indexed(index.saturating_add(8))
    } else {
        cell.foreground
    }
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "ownership transfer is intentional for compatibility palette updates"
)]
fn native_color_to_rgba(
    color: Color,
    default: [u8; 4],
    ansi_palette: Option<[Color; 16]>,
    indexed_palette: Option<[Option<Color>; 256]>,
) -> [u8; 4] {
    match color {
        Color::Indexed(index @ 0..=15) => ansi_palette
            .and_then(|palette| palette.get(usize::from(index)).copied())
            .map_or_else(
                || color_to_rgba(color, default),
                |color| color_to_rgba(color, default),
            ),
        Color::Indexed(index) => indexed_palette
            .and_then(|palette| palette.get(usize::from(index)).copied().flatten())
            .map_or_else(
                || color_to_rgba(color, default),
                |color| color_to_rgba(color, default),
            ),
        color => color_to_rgba(color, default),
    }
}

fn text_contrast_adjusted_foreground(
    foreground: [u8; 4],
    background: [u8; 4],
    min_ratio: f64,
) -> Option<[u8; 4]> {
    if foreground[..3] == background[..3] {
        return None;
    }

    let current_ratio = contrast_ratio(foreground, background);
    if current_ratio >= min_ratio {
        return None;
    }

    let black = [0, 0, 0, foreground[3]];
    let white = [u8::MAX, u8::MAX, u8::MAX, foreground[3]];
    let target = if contrast_ratio(black, background) > contrast_ratio(white, background) {
        black
    } else {
        white
    };
    if contrast_ratio(target, background) < min_ratio {
        return Some(target);
    }

    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..16 {
        let mid = f64::midpoint(low, high);
        let candidate = interpolate_rgba(foreground, target, mid);
        if contrast_ratio(candidate, background) >= min_ratio {
            high = mid;
        } else {
            low = mid;
        }
    }

    Some(interpolate_rgba(foreground, target, high))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn interpolate_rgba(from: [u8; 4], to: [u8; 4], amount: f64) -> [u8; 4] {
    let amount = amount.clamp(0.0, 1.0);
    [
        (f64::from(from[0]) + (f64::from(to[0]) - f64::from(from[0])) * amount).round() as u8,
        (f64::from(from[1]) + (f64::from(to[1]) - f64::from(from[1])) * amount).round() as u8,
        (f64::from(from[2]) + (f64::from(to[2]) - f64::from(from[2])) * amount).round() as u8,
        from[3],
    ]
}

fn contrast_ratio(foreground: [u8; 4], background: [u8; 4]) -> f64 {
    let foreground_luminance = relative_luminance(foreground);
    let background_luminance = relative_luminance(background);
    let light = foreground_luminance.max(background_luminance);
    let dark = foreground_luminance.min(background_luminance);
    (light + 0.05) / (dark + 0.05)
}

fn relative_luminance(color: [u8; 4]) -> f64 {
    let red = linear_srgb_component(color[0]);
    let green = linear_srgb_component(color[1]);
    let blue = linear_srgb_component(color[2]);
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

fn linear_srgb_component(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn rgba_to_color(rgba: [u8; 4]) -> Color {
    if rgba[3] == u8::MAX {
        Color::Rgb(rgba[0], rgba[1], rgba[2])
    } else {
        Color::Rgba(rgba[0], rgba[1], rgba[2], rgba[3])
    }
}

fn visual_bell_color_from_foreground(foreground: Color, default_foreground: Color) -> Color {
    match foreground {
        Color::Default => default_foreground,
        color => color,
    }
}

fn visual_bell_background_cells(
    snapshot: &TerminalRenderSnapshot,
    rows: u16,
    columns: u16,
    color: Color,
    intensity: f64,
    background_rgba: [u8; 4],
) -> Vec<RenderCell> {
    let occupied_cells: HashSet<(u16, u16)> = snapshot
        .iter_cells()
        .map(|cell| (cell.row, cell.column))
        .collect();
    let mut cells = Vec::new();
    let background = blend_visual_bell_color(
        Color::Default,
        color,
        background_rgba,
        intensity,
    );

    for row in 0..rows {
        for column in 0..columns {
            if occupied_cells.contains(&(row, column)) {
                continue;
            }

            cells.push(RenderCell::new(row, column, " ").with_style(RenderStyle {
                foreground: Color::Default,
                background,
                ..RenderStyle::default()
            }));
        }
    }

    cells
}

fn visual_bell_color_from_snapshot(
    snapshot: &TerminalRenderSnapshot,
    configured_color: Option<Color>,
    default_foreground: Color,
) -> Color {
    if let Some(color) = configured_color {
        return color;
    }

    snapshot
        .iter_cells()
        .map(|cell| visual_bell_color_from_foreground(cell.foreground, default_foreground))
        .find(|color| *color != Color::Rgb(255, 255, 255))
        .unwrap_or(default_foreground)
}

fn visual_bell_cursor_base_color(
    snapshot: &TerminalRenderSnapshot,
    force_reverse_video_cursor: bool,
) -> Color {
    if let Some(color) = snapshot.cursor_color() {
        return color;
    }

    if !force_reverse_video_cursor {
        return Color::Default;
    }

    let Some(cursor) = snapshot.cursor() else {
        return Color::Default;
    };

    snapshot
        .iter_cells()
        .find(|cell| cell.row == cursor.row && cell.column == cursor.column)
        .map_or(Color::Default, visual_bell_effective_cell_foreground)
}

fn visual_bell_effective_cell_foreground(cell: &RenderCell) -> Color {
    let foreground = color_to_rgba(cell.foreground, DEFAULT_RENDER_FOREGROUND_RGBA);
    let background = color_to_rgba(cell.background, DEFAULT_RENDER_BACKGROUND_RGBA);
    let foreground = if cell.inverse { background } else { foreground };
    let foreground = if cell.faint {
        [
            foreground[0] / 2,
            foreground[1] / 2,
            foreground[2] / 2,
            foreground[3],
        ]
    } else {
        foreground
    };

    if foreground[3] == u8::MAX {
        Color::Rgb(foreground[0], foreground[1], foreground[2])
    } else {
        Color::Rgba(foreground[0], foreground[1], foreground[2], foreground[3])
    }
}

fn visual_bell_intensity(visual_bell: NativeVisualBell, elapsed: Duration) -> Option<f64> {
    let fade_in = Duration::from_millis(visual_bell.fade_in_duration_ms);
    let fade_out = Duration::from_millis(visual_bell.fade_out_duration_ms);
    let total_duration = visual_bell.total_duration();
    if total_duration.is_zero() || elapsed >= total_duration {
        return None;
    }

    if !fade_in.is_zero() && elapsed < fade_in {
        let progress = duration_progress(elapsed, fade_in);
        return Some(easing_value(visual_bell.fade_in_function, progress));
    }

    if fade_out.is_zero() {
        return Some(1.0);
    }

    let progress = duration_progress(elapsed.saturating_sub(fade_in), fade_out);
    Some(1.0 - easing_value(visual_bell.fade_out_function, progress))
}

fn snapshot_has_active_text_blink(
    snapshot: &TerminalRenderSnapshot,
    regular_blink_active: bool,
    rapid_blink_active: bool,
) -> bool {
    snapshot.iter_cells().any(|cell| {
        cell.blink
            && ((regular_blink_active && !cell.rapid_blink)
                || (rapid_blink_active && cell.rapid_blink))
    })
}

fn inline_image_may_animate(image: &RenderInlineImage) -> bool {
    image.image_format == InlineImageFormat::Encoded
        && (image.data.starts_with(b"GIF87a") || image.data.starts_with(b"GIF89a"))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "visual-bell configuration and timing are millisecond-based, so whole-millisecond quantization is deliberate"
)]
fn duration_progress(elapsed: Duration, duration: Duration) -> f64 {
    let duration_ms = duration.as_millis();
    if duration_ms == 0 {
        return 1.0;
    }

    let elapsed_ms = elapsed.as_millis().min(duration_ms);
    elapsed_ms as f64 / duration_ms as f64
}

fn blend_visual_bell_color(
    base_color: Color,
    bell_color: Color,
    default_base_color: [u8; 4],
    intensity: f64,
) -> Color {
    if intensity >= 1.0 {
        return bell_color;
    }
    if intensity <= 0.0 {
        return base_color;
    }

    let base = color_to_rgba(base_color, default_base_color);
    let bell = color_to_rgba(bell_color, DEFAULT_RENDER_FOREGROUND_RGBA);
    let red = blend_visual_bell_channel(base[0], bell[0], intensity);
    let green = blend_visual_bell_channel(base[1], bell[1], intensity);
    let blue = blend_visual_bell_channel(base[2], bell[2], intensity);
    let alpha = blend_visual_bell_channel(base[3], bell[3], intensity);
    if alpha == u8::MAX {
        Color::Rgb(red, green, blue)
    } else {
        Color::Rgba(red, green, blue, alpha)
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn blend_visual_bell_channel(base: u8, bell: u8, intensity: f64) -> u8 {
    (f64::from(base) + (f64::from(bell) - f64::from(base)) * intensity.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, f64::from(u8::MAX)) as u8
}

fn native_ansi_palette_to_rgba(colors: [Color; 16]) -> [[u8; 4]; 16] {
    colors.map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA))
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "ownership transfer is intentional for compatibility palette updates"
)]
fn native_indexed_palette_to_rgba(colors: [Option<Color>; 256]) -> [Option<[u8; 4]>; 256] {
    colors.map(|color| color.map(|color| color_to_rgba(color, DEFAULT_RENDER_FOREGROUND_RGBA)))
}

fn inactive_pane_color(
    role: RenderCellColorRole,
    color: Color,
    hsb: NativeInactivePaneHsb,
    foreground: Color,
    background: Color,
) -> Color {
    let default = match role {
        RenderCellColorRole::Foreground | RenderCellColorRole::Underline => {
            color_to_rgba(foreground, DEFAULT_RENDER_FOREGROUND_RGBA)
        }
        RenderCellColorRole::Background => color_to_rgba(background, DEFAULT_RENDER_BACKGROUND_RGBA),
    };

    hsb_color(color, default, hsb)
}

fn foreground_text_hsb_color(
    role: RenderCellColorRole,
    color: Color,
    hsb: NativeInactivePaneHsb,
) -> Color {
    match role {
        RenderCellColorRole::Foreground | RenderCellColorRole::Underline => {
            hsb_color(color, DEFAULT_RENDER_FOREGROUND_RGBA, hsb)
        }
        RenderCellColorRole::Background => color,
    }
}

fn text_background_opacity_color(
    role: RenderCellColorRole,
    color: Color,
    opacity: NativeTextBackgroundOpacity,
) -> Color {
    match role {
        RenderCellColorRole::Background => non_default_background_with_opacity(color, opacity),
        RenderCellColorRole::Foreground | RenderCellColorRole::Underline => color,
    }
}

fn non_default_background_with_opacity(
    color: Color,
    opacity: NativeTextBackgroundOpacity,
) -> Color {
    let alpha = opacity.as_alpha();
    match color {
        Color::Default => Color::Default,
        Color::Rgb(red, green, blue) | Color::Rgba(red, green, blue, _) => {
            Color::Rgba(red, green, blue, alpha)
        }
        Color::Indexed(_) => {
            let [red, green, blue, _] = color_to_rgba(color, DEFAULT_RENDER_BACKGROUND_RGBA);
            Color::Rgba(red, green, blue, alpha)
        }
    }
}

fn window_background_opacity_color(
    role: RenderCellColorRole,
    color: Color,
    opacity: NativeTextBackgroundOpacity,
    background: Color,
) -> Color {
    match role {
        RenderCellColorRole::Background => {
            default_background_with_opacity(color, opacity, background)
        }
        RenderCellColorRole::Foreground | RenderCellColorRole::Underline => color,
    }
}

fn default_background_with_opacity(
    color: Color,
    opacity: NativeTextBackgroundOpacity,
    background: Color,
) -> Color {
    match color {
        Color::Default => {
            let [red, green, blue, _] = color_to_rgba(background, DEFAULT_RENDER_BACKGROUND_RGBA);
            Color::Rgba(red, green, blue, opacity.as_alpha())
        }
        color => color,
    }
}

fn hsb_color(color: Color, default: [u8; 4], hsb: NativeInactivePaneHsb) -> Color {
    match color {
        Color::Rgb(red, green, blue) => {
            let [red, green, blue] = transform_rgb_hsb(red, green, blue, hsb);
            Color::Rgb(red, green, blue)
        }
        Color::Rgba(red, green, blue, alpha) => {
            let [red, green, blue] = transform_rgb_hsb(red, green, blue, hsb);
            Color::Rgba(red, green, blue, alpha)
        }
        Color::Default | Color::Indexed(_) => {
            let [red, green, blue, alpha] = color_to_rgba(color, default);
            let [red, green, blue] = transform_rgb_hsb(red, green, blue, hsb);
            if alpha == 255 {
                Color::Rgb(red, green, blue)
            } else {
                Color::Rgba(red, green, blue, alpha)
            }
        }
    }
}

fn transform_rgb_hsb(red: u8, green: u8, blue: u8, hsb: NativeInactivePaneHsb) -> [u8; 3] {
    let red_channel = red;
    let green_channel = green;
    let max_channel = red.max(green).max(blue);
    let red = f64::from(red) / 255.0;
    let green = f64::from(green) / 255.0;
    let blue = f64::from(blue) / 255.0;

    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let delta = max - min;
    let hue = if delta <= f64::EPSILON {
        0.0
    } else if max_channel == red_channel {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if max_channel == green_channel {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    let saturation = if max <= f64::EPSILON {
        0.0
    } else {
        delta / max
    };
    let value = max;

    let hue = (hue * hsb.hue.as_f64()).rem_euclid(360.0);
    let saturation = (saturation * hsb.saturation.as_f64()).clamp(0.0, 1.0);
    let value = (value * hsb.brightness.as_f64()).clamp(0.0, 1.0);

    hsv_to_rgb(hue, saturation, value)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let chroma = value * saturation;
    let hue_sector = hue / 60.0;
    let x = chroma * (1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = if hue_sector < 1.0 {
        (chroma, x, 0.0)
    } else if hue_sector < 2.0 {
        (x, chroma, 0.0)
    } else if hue_sector < 3.0 {
        (0.0, chroma, x)
    } else if hue_sector < 4.0 {
        (0.0, x, chroma)
    } else if hue_sector < 5.0 {
        (x, 0.0, chroma)
    } else {
        (chroma, 0.0, x)
    };
    let m = value - chroma;

    [
        round_rgb_component(red + m),
        round_rgb_component(green + m),
        round_rgb_component(blue + m),
    ]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn round_rgb_component(component: f64) -> u8 {
    (component.mul_add(255.0, 1e-9)).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
const fn tab_bar_pixel_height() -> u32 {
    TAB_BAR_ROWS as u32 * CELL_HEIGHT
}

#[cfg(test)]
fn tab_bar_tab_label(
    position: usize,
    tab_id: rterm_core::TabId,
    pane_count: usize,
    active: bool,
    title: Option<&str>,
    progress: PaneProgress,
) -> String {
    let label = tab_bar_tab_label_segments(
        position,
        tab_id,
        pane_count,
        active,
        title,
        progress,
        TabBarTabLabelOptions {
            show_tab_index: true,
            zero_based_tab_index: false,
            show_close_button: true,
        },
    );
    format!("{}{}{}", label.prefix, label.title, label.suffix)
}

struct TabBarTabLabelSegments {
    prefix: String,
    title: String,
    suffix: String,
}

struct TabBarVisibleTabLayout {
    position: usize,
    tab_id: rterm_core::TabId,
    active: bool,
    hovered: bool,
    start_column: u16,
    end_column: u16,
    left_edge_end_column: u16,
    prefix_end_column: u16,
    title_end_column: u16,
    suffix_end_column: u16,
    left_edge: Option<Vec<NativeFormatItem>>,
    label: TabBarTabLabelSegments,
    title: NativeTabTitle,
    right_edge: Option<Vec<NativeFormatItem>>,
    close_column: Option<u16>,
}

impl TabBarVisibleTabLayout {
    fn reposition(&mut self, start_column: u16) {
        let previous_start = self.start_column;
        let translate = |column: u16| {
            start_column.saturating_add(column.saturating_sub(previous_start))
        };
        self.start_column = start_column;
        self.end_column = translate(self.end_column);
        self.left_edge_end_column = translate(self.left_edge_end_column);
        self.prefix_end_column = translate(self.prefix_end_column);
        self.title_end_column = translate(self.title_end_column);
        self.suffix_end_column = translate(self.suffix_end_column);
        self.close_column = self.close_column.map(translate);
    }
}

struct TabBarVisibleLayout {
    tabs: Vec<TabBarVisibleTabLayout>,
    leading_overflow_column: Option<u16>,
    overflow_column: Option<u16>,
    new_tab_start_column: Option<u16>,
    new_tab_end_column: Option<u16>,
    generation: u64,
}

// The tab formatter receives owned information for compatibility with the
// existing WezTerm-style hook API.  Build the expensive window-wide pieces
// once per layout and clone them into each event instead of recomputing the
// full effective config and all tab metadata for every tab/two-pass call.
struct TabBarTitleContext {
    config: NativeConfigView,
    tabs: Vec<NativeTabInformation>,
    active_pane_info: Vec<NativePaneInformation>,
}

#[derive(Clone, Copy)]
struct TabBarDrag {
    source_tab_id: rterm_core::TabId,
    pressed_pixel_x: f64,
    moved: bool,
}

#[derive(Clone, Copy)]
struct TabBarTabLabelOptions {
    show_tab_index: bool,
    zero_based_tab_index: bool,
    show_close_button: bool,
}

#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
struct TabBarSegmentStyle {
    foreground: Color,
    background: Color,
    underline_color: Color,
    bold: bool,
    faint: bool,
    italic: bool,
    blink: bool,
    rapid_blink: bool,
    inverse: bool,
    conceal: bool,
    strikethrough: bool,
    overline: bool,
    underline_style: UnderlineStyle,
}

const fn tab_bar_segment_style(
    foreground: Color,
    background: Color,
    bold: bool,
) -> TabBarSegmentStyle {
    TabBarSegmentStyle {
        foreground,
        background,
        underline_color: Color::Default,
        bold,
        faint: false,
        italic: false,
        blink: false,
        rapid_blink: false,
        inverse: false,
        conceal: false,
        strikethrough: false,
        overline: false,
        underline_style: UnderlineStyle::None,
    }
}

fn tab_bar_item_segment_style(
    colors: NativeTabBarItemColors,
    default_foreground: Color,
    default_background: Color,
    bold: bool,
) -> TabBarSegmentStyle {
    let mut style = tab_bar_segment_style(
        match colors.fg_color {
            Some(color) => color,
            None => default_foreground,
        },
        match colors.bg_color {
            Some(color) => color,
            None => default_background,
        },
        bold,
    );
    if let Some(intensity) = colors.intensity {
        match intensity {
            NativeFormatIntensity::Normal => {
                style.bold = false;
                style.faint = false;
            }
            NativeFormatIntensity::Bold => {
                style.bold = true;
                style.faint = false;
            }
            NativeFormatIntensity::Half => {
                style.bold = false;
                style.faint = true;
            }
        }
    }
    if let Some(underline) = colors.underline {
        style.underline_style = underline_style_for_native_format(underline);
    }
    if let Some(italic) = colors.italic {
        style.italic = italic;
    }
    if let Some(strikethrough) = colors.strikethrough {
        style.strikethrough = strikethrough;
    }
    style
}
