use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use super::*;

pub(super) struct AgentRow {
    pub(super) pane_id: String,
    pub(super) status: crate::api::schema::AgentStatus,
    pub(super) focused: bool,
    pub(super) rows: Vec<Vec<crate::ui::ResolvedToken>>,
}

pub(super) fn ordered_agent_pane_ids(
    snapshot: &ClientShellSnapshot,
    sort: crate::config::AgentPanelSortConfig,
) -> Vec<String> {
    if snapshot.agent_view_label.is_some() {
        return snapshot
            .agent_order
            .iter()
            .filter(|pane_id| {
                snapshot
                    .agents
                    .iter()
                    .any(|agent| agent.pane_id == pane_id.as_str())
            })
            .cloned()
            .collect();
    }
    let mut agents = snapshot.agents.iter().collect::<Vec<_>>();
    if sort == crate::config::AgentPanelSortConfig::Priority {
        agents.sort_by_key(|agent| {
            (
                std::cmp::Reverse(status_priority(agent.agent_status)),
                std::cmp::Reverse(agent.state_change_seq),
            )
        });
    }
    agents
        .into_iter()
        .map(|agent| agent.pane_id.clone())
        .collect()
}

pub(super) fn render_agent_panel(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
) {
    if !render_agent_panel_header(
        buffer,
        area,
        snapshot.agent_view_label.as_deref(),
        config,
        hits,
    ) {
        return;
    }

    let rows = agent_rows(snapshot, config, None);
    render_agent_list(
        buffer,
        area,
        &rows,
        snapshot
            .agent_view_label
            .as_ref()
            .map(|_| " no matching agents"),
        config,
        agent_scroll,
        hits,
        |row| row.rows.len(),
        |buffer, rect, row, hits| {
            hits.agents.push((rect, row.pane_id.clone()));
            render_agent_row(buffer, rect, row, config);
        },
    );
}

pub(super) fn render_agent_panel_header(
    buffer: &mut Buffer,
    area: Rect,
    agent_view_label: Option<&str>,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) -> bool {
    if area.height == 0 {
        return false;
    }
    put_text(
        buffer,
        area.x,
        area.y,
        area.width,
        &"─".repeat(area.width as usize),
        Style::default().fg(config.palette.surface_dim),
    );
    if area.height < 2 {
        return false;
    }
    put_text(
        buffer,
        area.x,
        area.y + 1,
        area.width,
        " agents",
        Style::default()
            .fg(config.palette.overlay0)
            .add_modifier(Modifier::BOLD),
    );
    let sort_label = agent_view_label.unwrap_or(match config.agent_panel_sort {
        crate::config::AgentPanelSortConfig::Spaces => "grouped",
        crate::config::AgentPanelSortConfig::Priority => "priority",
    });
    let sort_width = display_width(sort_label).min(area.width as usize) as u16;
    let sort_rect = Rect::new(
        area.right().saturating_sub(sort_width),
        area.y + 1,
        sort_width,
        1,
    );
    hits.agent_sort_toggle = if config.mouse_capture && agent_view_label.is_none() {
        sort_rect
    } else {
        Rect::default()
    };
    put_text(
        buffer,
        sort_rect.x,
        sort_rect.y,
        sort_rect.width,
        sort_label,
        Style::default()
            .fg(if agent_view_label.is_some() {
                config.palette.accent
            } else {
                config.palette.overlay0
            })
            .add_modifier(Modifier::BOLD),
    );
    true
}

pub(super) fn render_agent_list<T>(
    buffer: &mut Buffer,
    area: Rect,
    rows: &[T],
    empty_message: Option<&str>,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
    row_lines: impl Fn(&T) -> usize,
    mut render_row: impl FnMut(&mut Buffer, Rect, &T, &mut ShellHitMap),
) {
    let body = Rect::new(
        area.x,
        area.y.saturating_add(3),
        area.width,
        area.height.saturating_sub(3),
    );
    hits.agent_body = body;
    if body.is_empty() || rows.is_empty() {
        *agent_scroll = 0;
        if let Some(message) = empty_message.filter(|_| !body.is_empty()) {
            put_text(
                buffer,
                body.x,
                body.y,
                body.width,
                message,
                Style::default()
                    .fg(config.palette.overlay0)
                    .add_modifier(Modifier::DIM),
            );
        }
        return;
    }

    let row_heights = rows
        .iter()
        .map(|row| row_lines(row).max(1).min(u16::MAX as usize) as u16)
        .collect::<Vec<_>>();
    let gaps = rows
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if index + 1 < rows.len() {
                config.agents.row_gap
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    let metrics =
        super::scroll::list_scroll_metrics(&row_heights, &gaps, body.height, *agent_scroll);
    hits.agent_max_scroll = metrics.max_offset_from_bottom;
    hits.agent_scroll_metrics = Some(metrics);
    *agent_scroll = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let show_scrollbar = metrics.max_offset_from_bottom > 0 && body.width > 1;
    let content_width = body.width.saturating_sub(u16::from(show_scrollbar));
    let mut y = body.y;
    for (index, row) in rows.iter().enumerate().skip(*agent_scroll) {
        let height = row_heights[index].min(body.height);
        if y.saturating_add(height) > body.bottom() {
            break;
        }
        let rect = Rect::new(body.x, y, content_width, height);
        render_row(buffer, rect, row, hits);
        y = y
            .saturating_add(height)
            .saturating_add(if index + 1 < rows.len() {
                config.agents.row_gap
            } else {
                0
            });
    }

    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.agent_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, &config.palette);
    }
}

pub(super) fn agent_rows(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    machine: Option<&str>,
) -> Vec<AgentRow> {
    ordered_agent_pane_ids(snapshot, config.agent_panel_sort)
        .into_iter()
        .filter_map(|pane_id| {
            let agent = snapshot
                .agents
                .iter()
                .find(|agent| agent.pane_id == pane_id)?;
            let workspace = snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == agent.workspace_id)?;
            let tab = snapshot.tabs.iter().find(|tab| tab.tab_id == agent.tab_id);
            let pane = snapshot
                .panes
                .iter()
                .find(|pane| pane.pane_id == agent.pane_id);
            let tab_count = snapshot
                .tabs
                .iter()
                .filter(|candidate| candidate.workspace_id == agent.workspace_id)
                .count();
            let tab_label = tab
                .filter(|tab| tab_count > 1 || tab.custom_label)
                .map(|tab| tab.label.as_str());
            let agent_label = agent
                .display_agent
                .as_deref()
                .or(agent.name.as_deref())
                .or(agent.agent.as_deref())
                .or(agent.title.as_deref());
            let labels = agent
                .state_labels
                .iter()
                .cloned()
                .collect::<HashMap<_, _>>();
            let tokens = agent.tokens.iter().cloned().collect::<HashMap<_, _>>();
            let state_text = labels
                .get(status_text(agent.agent_status))
                .map(String::as_str)
                .unwrap_or_else(|| sidebar_status_text(agent.agent_status));
            let canonical_agent = agent
                .agent
                .as_deref()
                .and_then(crate::detect::parse_agent_label);
            let rows = crate::ui::sidebar_agent_rows(
                &config.agents,
                crate::ui::AgentTokenContext {
                    machine,
                    workspace: &workspace.label,
                    tab: tab_label,
                    pane: agent
                        .title
                        .as_deref()
                        .or_else(|| pane.and_then(|pane| pane.label.as_deref())),
                    agent_label,
                    terminal_title: agent.terminal_title.as_deref(),
                    terminal_title_stripped: agent.terminal_title_stripped.as_deref(),
                    canonical_agent,
                    tokens: &tokens,
                },
                state_text,
            );
            Some(AgentRow {
                pane_id: agent.pane_id.clone(),
                status: agent.agent_status,
                focused: agent.focused,
                rows,
            })
        })
        .collect()
}

pub(super) fn render_agent_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &AgentRow,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    let outline = config.agent_selection_style == crate::config::AgentSelectionStyle::Outline;
    let row_style = if row.focused && !outline {
        Style::default().bg(palette.active_row_bg)
    } else {
        Style::default()
    };
    let name_style = if row.focused {
        Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(palette.subtext0)
            .add_modifier(Modifier::BOLD)
    };
    let status_style = Style::default()
        .fg(status_color(row.status, palette))
        .add_modifier(if row.focused {
            Modifier::empty()
        } else {
            Modifier::DIM
        });
    let secondary = Style::default()
        .fg(palette.overlay0)
        .add_modifier(Modifier::DIM);
    let icon = (
        status_icon(row.status, config.status_indicators),
        Style::default().fg(status_color(row.status, palette)),
    );
    let rows = if row.rows.is_empty() {
        vec![vec![crate::ui::ResolvedToken {
            kind: crate::ui::ResolvedTokenKind::StateIcon,
            style: Default::default(),
        }]]
    } else {
        row.rows.clone()
    };
    // Select the last actual content line, not optional group headings or
    // zero-width spacer tokens. This gives grouped plugin entries the same
    // one-line marker without adding rows or changing their mouse hitboxes.
    let outline_line = (outline && row.focused)
        .then(|| {
            rows.iter()
                .rposition(|tokens| tokens.iter().any(|token| token.kind.has_visible_content()))
        })
        .flatten();
    for (index, tokens) in rows.iter().take(rect.height as usize).enumerate() {
        let indent = if index == 0 { 1 } else { 3 };
        // Reserve the right edge on every outline-mode row so focusing an
        // entry never changes title truncation. The left edge is existing padding.
        let content_width = rect.width.saturating_sub(u16::from(outline));
        let mut spans = vec![ratatui::text::Span::raw(" ".repeat(indent))];
        spans.extend(crate::ui::resolved_token_spans(
            tokens,
            icon,
            status_style,
            name_style,
            secondary,
            secondary,
            palette,
            true,
            content_width.saturating_sub(indent as u16) as usize,
        ));
        Paragraph::new(Line::from(spans)).style(row_style).render(
            Rect::new(rect.x, rect.y + index as u16, content_width, 1),
            buffer,
        );
        if outline_line == Some(index) && rect.width >= 2 {
            let y = rect.y + index as u16;
            let style = Style::default().fg(palette.accent);
            buffer[(rect.x, y)].set_symbol("[").set_style(style);
            buffer[(rect.right() - 1, y)]
                .set_symbol("]")
                .set_style(style);
        }
    }
}

fn put_text(buffer: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    for (offset, character) in text.chars().take(width as usize).enumerate() {
        if let Some(cell) = buffer.cell_mut((x + offset as u16, y)) {
            cell.set_char(character).set_style(style);
        }
    }
}

fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

fn sidebar_status_text(status: crate::api::schema::AgentStatus) -> &'static str {
    use crate::api::schema::AgentStatus;
    match status {
        AgentStatus::Blocked => "blocked",
        AgentStatus::Done => "done",
        AgentStatus::Working => "working",
        AgentStatus::Idle | AgentStatus::Unknown => "idle",
    }
}

#[cfg(test)]
mod outline_tests {
    use super::*;
    use crate::config::{AgentSelectionStyle, Config};
    use crate::ui::{ResolvedToken, ResolvedTokenKind};
    use ratatui::style::Color;

    const BACKGROUND: Color = Color::Rgb(250, 250, 250);

    fn content(text: &str) -> Vec<ResolvedToken> {
        vec![ResolvedToken {
            kind: ResolvedTokenKind::Custom(text.into()),
            style: Default::default(),
        }]
    }

    fn fixture(grouped: bool) -> AgentRow {
        let mut rows = Vec::new();
        if grouped {
            rows.push(content("Hermes"));
        }
        rows.push(content("◉ Codex: sidebar"));
        if grouped {
            rows.push(content("\u{200b}"));
        }
        AgentRow {
            pane_id: "fixture".into(),
            status: crate::api::schema::AgentStatus::Working,
            focused: true,
            rows,
        }
    }

    fn draw(row: &AgentRow, style: AgentSelectionStyle, rect: Rect) -> Buffer {
        let mut config = ClientShellConfig::from_config(&Config::default());
        config.agent_selection_style = style;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 36, 8));
        buffer.set_style(buffer.area, Style::default().bg(BACKGROUND));
        render_agent_row(&mut buffer, rect, row, &config);
        buffer
    }

    #[test]
    fn outline_config_is_opt_in_and_rejects_typos() {
        let defaults: Config = toml::from_str("").unwrap();
        assert_eq!(defaults.ui.agent_selection_style, AgentSelectionStyle::Fill);
        let outline: Config = toml::from_str("[ui]\nagent_selection_style = 'outline'").unwrap();
        assert_eq!(
            outline.ui.agent_selection_style,
            AgentSelectionStyle::Outline
        );
        assert!(toml::from_str::<Config>("[ui]\nagent_selection_style = 'outlien'").is_err());
    }

    #[test]
    fn outline_marks_only_title_preserving_headings_spacers_and_background() {
        for grouped in [false, true] {
            let row = fixture(grouped);
            let rect = Rect::new(2, 1, 30, row.rows.len() as u16);
            let buffer = draw(&row, AgentSelectionStyle::Outline, rect);
            let title_y = rect.y + u16::from(grouped);
            assert_eq!(buffer[(rect.x, title_y)].symbol(), "[");
            assert_eq!(buffer[(rect.right() - 1, title_y)].symbol(), "]");
            let marked_rows = (rect.y..rect.bottom())
                .filter(|y| buffer[(rect.x, *y)].symbol() == "[")
                .count();
            assert_eq!(marked_rows, 1);
            assert!(buffer.content.iter().all(|cell| cell.bg == BACKGROUND));
            if grouped {
                assert_eq!(buffer[(rect.x + 1, rect.y)].symbol(), "H");
                assert_eq!(buffer[(rect.x, rect.bottom() - 1)].symbol(), " ");
            }
            let mut idle = row;
            idle.focused = false;
            let unfocused = draw(&idle, AgentSelectionStyle::Outline, rect);
            // Focusing changes only the two edge markers, not title text,
            // token styling, or truncation. Radar tokens use the secondary style.
            for y in rect.y..rect.bottom() {
                for x in rect.x + 1..rect.right() - 1 {
                    assert_eq!(buffer[(x, y)], unfocused[(x, y)]);
                }
            }
        }
    }

    #[test]
    fn outline_respects_narrow_clipped_and_unicode_rows() {
        let mut row = fixture(true);
        row.rows[1] = content("界界界界界界界界界界");
        for width in 0..=30 {
            let rect = Rect::new(2, 1, width, 3);
            let buffer = draw(&row, AgentSelectionStyle::Outline, rect);
            assert!(buffer.content.iter().all(|cell| cell.bg == BACKGROUND));
            if width >= 2 {
                assert_eq!(buffer[(rect.right() - 1, rect.y + 1)].symbol(), "]");
            }
            assert_eq!(buffer[(rect.right(), rect.y + 1)].symbol(), " ");
        }
        // A clipped title must not cause its heading to appear selected.
        let buffer = draw(&row, AgentSelectionStyle::Outline, Rect::new(2, 1, 30, 1));
        assert!(!buffer.content.iter().any(|cell| cell.symbol() == "["));
    }

    #[test]
    fn default_fill_still_covers_the_whole_entry() {
        let row = fixture(true);
        let rect = Rect::new(2, 1, 30, 3);
        let buffer = draw(&row, AgentSelectionStyle::Fill, rect);
        let palette = ClientShellConfig::from_config(&Config::default()).palette;
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                assert_eq!(buffer[(x, y)].bg, palette.active_row_bg);
            }
        }
    }

    #[test]
    #[ignore = "manual render artifact and non-gating scaling profile"]
    fn outline_selection_preview_and_scale() {
        for style in [AgentSelectionStyle::Fill, AgentSelectionStyle::Outline] {
            let mut row = fixture(true);
            row.rows[0][0].style.fg = Some(serde_json::from_str("\"#7c7f93\"").unwrap());
            row.rows[0][0].style.bold = Some(true);
            row.rows[0][0].style.dim = Some(false);
            row.rows[1][0].style.fg = Some(serde_json::from_str("\"#c78a1f\"").unwrap());
            row.rows[1][0].style.bold = Some(true);
            row.rows[1][0].style.dim = Some(false);
            let mut config = Config::default();
            config.theme.name = Some("catppuccin-latte".into());
            config.ui.agent_selection_style = style;
            let mut shell_config = ClientShellConfig::from_config(&config);
            shell_config.palette.active_row_bg = Color::Rgb(185, 205, 242);
            let mut buffer = Buffer::empty(Rect::new(0, 0, 36, 6));
            buffer.set_style(buffer.area, Style::default().bg(BACKGROUND));
            render_agent_row(&mut buffer, Rect::new(2, 1, 30, 3), &row, &shell_config);
            let cells = buffer
                .content
                .iter()
                .map(|cell| {
                    serde_json::json!({"text":cell.symbol(), "fg":format!("{:?}", cell.fg),
                    "bg":format!("{:?}", cell.bg), "modifier":format!("{:?}", cell.modifier)})
                })
                .collect::<Vec<_>>();
            println!(
                "OUTLINE_PREVIEW {}",
                serde_json::json!({"style":format!("{style:?}"),
                "width":buffer.area.width, "height":buffer.area.height, "cells":cells})
            );
        }
        // Fixed 36-column geometry with one focused agent, matching a sidebar.
        // Measure only the rendering loop; fixture construction is outside it.
        for count in [1, 15] {
            for style in [AgentSelectionStyle::Fill, AgentSelectionStyle::Outline] {
                let mut config = ClientShellConfig::from_config(&Config::default());
                config.agent_selection_style = style;
                let rows = (0..count)
                    .map(|index| {
                        let mut row = fixture(index % 3 == 0);
                        row.focused = index == 0;
                        row
                    })
                    .collect::<Vec<_>>();
                let mut buffer = Buffer::empty(Rect::new(0, 0, 36, 60));
                let start = std::time::Instant::now();
                for _ in 0..2000 {
                    for (index, row) in rows.iter().enumerate() {
                        render_agent_row(
                            &mut buffer,
                            Rect::new(0, (index * 4) as u16, 36, 3),
                            row,
                            &config,
                        );
                    }
                    std::hint::black_box(&buffer);
                }
                println!(
                    "OUTLINE_SCALE agents={count} style={style:?} us_per_frame={:.2}",
                    start.elapsed().as_secs_f64() * 1_000_000.0 / 2000.0
                );
            }
        }
    }
}
