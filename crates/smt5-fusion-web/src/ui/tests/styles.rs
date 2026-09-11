use std::collections::{BTreeMap, BTreeSet};

const SOURCE: &str = include_str!("../../../index.html");
const CSS: &str = include_str!("../../../style.css");
const THEME: &str = include_str!("../theme.rs");
const PAGE: &str = include_str!("../pages/calculator.rs");
const RESULT_PANEL: &str = include_str!("../components/result_panel.rs");

type Palette = BTreeMap<&'static str, &'static str>;

fn rules() -> Vec<(String, &'static str)> {
    CSS.split('}')
        .filter_map(|section| {
            let (heading, body) = section.rsplit_once('{')?;
            let selector = heading
                .rsplit('{')
                .next()?
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            Some((selector, body))
        })
        .collect()
}

fn declarations(body: &'static str) -> Palette {
    body.split(';')
        .filter_map(|declaration| {
            let (name, value) = declaration.split_once(':')?;
            Some((name.trim(), value.trim()))
        })
        .collect()
}

fn rule(selector: &str) -> Palette {
    let (_, body) = rules()
        .into_iter()
        .find(|(name, _)| name == selector)
        .unwrap_or_else(|| panic!("Missing CSS rule: {selector}"));
    declarations(body)
}

fn css_block(source: &'static str, selector: &str) -> &'static str {
    let (_, body) = source
        .split_once(&format!("{selector} {{"))
        .unwrap_or_else(|| panic!("Missing CSS block: {selector}"));
    let mut depth = 1;
    for (index, ch) in body.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return &body[..index];
        }
    }
    panic!("Unclosed CSS block: {selector}");
}

fn palette(dark: bool) -> Palette {
    let mut palette = rule(":root");
    if dark {
        palette.extend(rule(":root[data-theme=\"dark\"]"));
    }
    palette
}

fn value(name: &str, palette: &Palette) -> &'static str {
    let mut current = name;
    let mut seen = BTreeSet::new();
    loop {
        assert!(seen.insert(current), "Cyclic variable: {current}");
        let value = palette
            .get(current)
            .unwrap_or_else(|| panic!("Missing {current}"));
        if let Some(reference) = value
            .strip_prefix("var(")
            .and_then(|value| value.strip_suffix(')'))
        {
            current = reference;
        } else {
            return value;
        }
    }
}

fn rgb(hex: &str) -> [f64; 3] {
    let hex = hex.strip_prefix('#').expect("Expected a hex color");
    let full = match hex.len() {
        3 => hex
            .chars()
            .flat_map(|digit| [digit, digit])
            .collect::<String>(),
        6 => hex.to_owned(),
        _ => panic!("Invalid color: {hex}"),
    };
    [0, 2, 4]
        .map(|offset| u8::from_str_radix(&full[offset..offset + 2], 16).unwrap() as f64 / 255.0)
}

fn luminance(color: [f64; 3]) -> f64 {
    let linear = color.map(|channel| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    });
    linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
}

fn contrast(a: [f64; 3], b: [f64; 3]) -> f64 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn check_contrast(foreground: &str, background: &str, minimum: f64, palette: &Palette) {
    let ratio = contrast(
        rgb(value(foreground, palette)),
        rgb(value(background, palette)),
    );
    assert!(
        ratio >= minimum,
        "{foreground} on {background}: {ratio:.2} < {minimum}"
    );
}

#[test]
fn bootstrap_precedes_styles_and_wasm_and_does_not_write_preferences() {
    let start = SOURCE.find("<script id=\"theme-bootstrap\">").unwrap();
    let end = SOURCE[start..].find("</script>").unwrap() + start;
    let script = &SOURCE[start..end];
    for forbidden in ["setItem", "location", "pathname", "data-theme-transition"] {
        assert!(!script.contains(forbidden));
    }
    for (start, _) in SOURCE.match_indices("<link") {
        let tag = &SOURCE[start..start + SOURCE[start..].find('>').unwrap()];
        if ["rel=\"stylesheet\"", "rel=\"css\"", "rel=\"rust\""]
            .iter()
            .any(|value| tag.contains(value))
        {
            assert!(end < start);
        }
    }
}

#[test]
fn header_controls_share_outer_sizing_instead_of_text_height() {
    let shared = rule(".language-trigger, .theme-switch");
    assert_eq!(shared["height"], "2.75rem");
    assert_eq!(shared["min-height"], "44px");
    assert_eq!(rule(".header-controls")["align-items"], "center");
    for selector in [".language-trigger", ".theme-switch"] {
        let own = rule(selector);
        assert!(!own.contains_key("height") && !own.contains_key("min-height"));
        assert_eq!(own["border"], "1px solid var(--header-control-border)");
    }
    let language = rule(".language-trigger");
    let segment = rule(".theme-option span");
    assert_eq!(language["align-items"], "center");
    assert_eq!(language["padding"], "0 0.75rem");
    assert_eq!(segment["height"], "100%");
    assert!(!segment.contains_key("min-height"));
    assert_eq!(segment["align-items"], "center");
    assert_eq!(segment["padding"], "0 0.6rem");
}

#[test]
fn filled_skill_slots_use_separate_native_edit_and_remove_buttons() {
    let source = include_str!("../components/required_skills.rs");
    let (_, filled) = source.split_once("<div class=slot_class>").unwrap();
    let (edit, rest) = filled.split_once("</button>").unwrap();
    let (remove, rest) = rest.split_once("</button>").unwrap();
    assert_eq!(edit.matches("<button").count(), 1);
    assert!(edit.contains("class=\"skill-slot-edit\""));
    assert!(edit.contains("id=format!(\"skill-slot-{index}\")"));
    assert!(edit.contains("aria-haspopup=\"dialog\""));
    assert!(edit.contains("disabled=move || !state.can_edit_skills()"));
    assert!(edit.contains("on:click="));
    assert!(edit.contains("controller.open_skill_picker(index)"));
    assert_eq!(remove.matches("<button").count(), 1);
    assert!(remove.contains("class=\"icon-button\""));
    assert!(remove.contains("aria-label=i18n.remove_skill"));
    assert!(remove.contains("controller.remove_skill(index)"));
    assert!(!remove.contains("open_skill_picker"));
    assert!(rest.trim_start().starts_with("</div>"));
}

#[test]
fn skill_slot_edit_button_spans_the_row_beneath_the_remove_button() {
    let slot = rule(".skill-slot.filled");
    assert_eq!(slot["position"], "relative");
    assert_eq!(slot["padding"], "0");
    let edit = rule(".skill-slot-edit");
    assert_eq!(edit["flex"], "1 1 auto");
    assert_eq!(edit["align-self"], "stretch");
    assert_eq!(edit["min-width"], "0");
    assert_eq!(
        edit["padding"],
        "0.22rem calc(34px + 0.72rem) 0.22rem 0.36rem"
    );
    let remove = rule(".skill-slot .icon-button");
    assert_eq!(remove["position"], "absolute");
    assert_eq!(remove["top"], "50%");
    assert_eq!(remove["right"], "0.36rem");
    assert_eq!(remove["width"], "34px");
    assert_eq!(remove["transform"], "translateY(-50%)");
    for style in [rule(".skill-slot"), slot, edit, remove] {
        assert!(!style.contains_key("-webkit-tap-highlight-color"));
        assert!(!style.contains_key("user-select"));
        assert!(!style.contains_key("-webkit-user-select"));
    }
    let hover = css_block(CSS, "@media (hover: hover)");
    assert!(hover.contains(".skill-slot.filled:hover:has(.skill-slot-edit:not(:disabled))"));
    assert!(hover.contains(".skill-slot.empty:hover:not(:disabled)"));
    let hover = rule(
        ".skill-slot.filled:hover:has(.skill-slot-edit:not(:disabled)), .skill-slot.empty:hover:not(:disabled)",
    );
    assert_eq!(hover["background"], "var(--orange-soft)");
    assert_eq!(
        rule(".skill-option[aria-current=\"true\"]")["background"],
        "var(--green-soft)"
    );
}

#[test]
fn pickers_share_fixed_geometry_and_scroll_only_the_results() {
    let backdrop = rule(".modal-backdrop");
    assert_eq!(backdrop["position"], "fixed");
    assert_eq!(backdrop["inset"], "0");
    assert_eq!(backdrop["overflow"], "hidden");
    let viewport = rule(".picker-viewport");
    assert_eq!(viewport["position"], "absolute");
    assert_eq!(viewport["height"], "100svh");
    assert_eq!(viewport["place-items"], "center");
    assert_eq!(viewport["overflow"], "hidden");
    for selector in [".error-notice", ".language-options", ".target-options"] {
        assert!(
            backdrop["z-index"].parse::<u32>().unwrap()
                > rule(selector)["z-index"].parse::<u32>().unwrap()
        );
    }
    for edge in ["top", "right", "bottom", "left"] {
        assert!(viewport["padding"].contains(&format!("env(safe-area-inset-{edge})")));
    }
    let panel = rule(".picker-panel");
    assert_eq!(panel["width"], "min(960px, 100%)");
    assert_eq!(panel["height"], "min(780px, 100%)");
    assert_eq!(panel["min-height"], "0");
    assert_eq!(panel["overflow"], "hidden");
    assert!(!panel.contains_key("max-height"));
    assert_eq!(CSS.matches(".picker-panel {").count(), 1);
    assert_eq!(rule(".picker-heading")["flex"], "0 0 auto");
    assert_eq!(rule(".skill-picker-filters")["flex"], "0 0 auto");
    assert_eq!(rule(".source-search")["flex"], "0 0 auto");
    let close = rule(".picker-heading .icon-button");
    assert_eq!(close["width"], "44px");
    assert_eq!(close["height"], "44px");
    assert_eq!(close["flex"], "0 0 44px");
    let list = rule(".picker-list");
    assert_eq!(list["flex"], "1 1 auto");
    assert_eq!(list["overflow-y"], "auto");
    assert_eq!(list["overscroll-behavior"], "contain");
    assert_eq!(list["min-height"], "0");
    assert_eq!(list["scrollbar-gutter"], "stable");
    for selector in [".skill-option", ".option-card"] {
        assert_eq!(rule(selector)["flex"], "0 0 auto");
    }
}

#[test]
fn backdrop_coverage_is_independent_of_keyboard_viewport_bounds() {
    let dialog = include_str!("../components/picker_dialog.rs");
    let backdrop_tag = dialog
        .split_once("class=\"modal-backdrop\"")
        .unwrap()
        .1
        .split_once('>')
        .unwrap()
        .0;
    assert!(!backdrop_tag.contains("style="));
    assert!(!backdrop_tag.contains("viewport.get()"));
    assert!(backdrop_tag.contains("on_close.run(())"));
    let viewport_tag = dialog
        .split_once("class=\"picker-viewport\"")
        .unwrap()
        .1
        .split_once('>')
        .unwrap()
        .0;
    assert!(viewport_tag.contains("style=move || viewport.get().map(Viewport::style)"));
    assert_eq!(
        dialog
            .matches("viewport.get().map(Viewport::style)")
            .count(),
        1
    );
    let backdrop = rule(".modal-backdrop");
    assert_eq!(backdrop["position"], "fixed");
    assert_eq!(backdrop["inset"], "0");
    assert_eq!(backdrop["background"], "var(--backdrop)");
    for property in [
        "width",
        "height",
        "max-height",
        "padding",
        "transform",
        "backdrop-filter",
    ] {
        assert!(!backdrop.contains_key(property));
    }
    assert_eq!(rule("html")["scrollbar-gutter"], "stable");
    assert_eq!(CSS.matches(".modal-backdrop {").count(), 1);
    let viewport = rule(".picker-viewport");
    assert!(!viewport.contains_key("background"));
    assert!(!viewport.contains_key("backdrop-filter"));
    let mobile = css_block(CSS, "@media (max-width: 880px)");
    assert_eq!(
        declarations(css_block(mobile, ".picker-viewport"))["--picker-inset"],
        "0.5rem"
    );
}

#[test]
fn both_pickers_use_the_same_modal_lifecycle_outside_the_inert_workspace() {
    let dialog = include_str!("../components/picker_dialog.rs");
    for source in [
        include_str!("../components/skill_picker.rs"),
        include_str!("../components/source_picker.rs"),
    ] {
        assert!(source.contains("<PickerDialog"));
        assert!(source.contains("class=\"picker-list\""));
        assert!(source.contains("focus_picker_on_open(&input)"));
        assert!(!source.contains("on:focusout"));
        assert!(!source.contains("on:mousedown"));
    }
    assert!(dialog.contains("aria-modal=\"true\""));
    assert!(dialog.contains("trap_tab(&event)"));
    assert!(dialog.contains("restore_focus(target)"));
    assert!(dialog.contains("event.key() == \"Escape\""));
    assert!(
        dialog.find("class=\"picker-heading\"").unwrap() < dialog.find("{children()}").unwrap()
    );
    let lock = rule("html:has(.modal-backdrop), body:has(.modal-backdrop)");
    assert_eq!(lock["overflow"], "hidden");
    assert_eq!(lock["overscroll-behavior"], "none");
    assert_eq!(PAGE.matches("inert=move || state.modal_open()").count(), 4);
    for picker in ["<SkillPicker />", "<SourcePicker />"] {
        assert!(PAGE.find("</main>").unwrap() < PAGE.find(picker).unwrap());
    }
    assert!(!include_str!("../components/route_tree.rs").contains("<SourcePicker"));
}

#[test]
fn only_explicit_searches_show_calculation_feedback() {
    let search = include_str!("../components/search_panel.rs");
    let source = include_str!("../components/source_picker.rs");
    let state = include_str!("../state.rs");

    assert!(search.contains("state.search_indicator_visible.get()"));
    assert!(search.contains("Message::SearchInProgress"));
    assert!(!search.contains("WorkerStatus::Loading"));
    assert!(!source.contains("spinner"));
    assert!(!source.contains("loading"));
    assert_eq!(state.matches("schedule_loading_indicator(").count(), 3);
    assert!(
        state.contains(
            "schedule_loading_indicator(move || state.reveal_search_indicator(request_id))"
        )
    );
}

#[test]
fn recipe_dialog_is_stable_while_its_contents_are_computed() {
    let source = include_str!("../components/source_picker.rs");

    assert!(source.contains("state.panel_path.get().is_some()"));
    assert!(source.contains("route_node_at_path(result.tree.as_ref()?, &path)"));
    assert!(source.contains("title_demon.map(|demon|"));
    assert!(source.contains("<Show when=move || state.options.get().is_some()>"));
    assert!(
        source.find("<PickerDialog").unwrap()
            < source.find("state.options.get().is_some()").unwrap()
    );
}

#[test]
fn route_skill_groups_and_recipe_metrics_fit_narrow_cards() {
    assert_eq!(rule(".skill-block")["display"], "grid");
    assert_eq!(rule(".skill-block")["gap"], "0.45rem");
    let badges = rule(".skill-badges");
    assert_eq!(badges["display"], "flex");
    assert_eq!(badges["flex-wrap"], "wrap");
    assert_eq!(badges["align-items"], "center");
    let label = rule(".skill-source-group > .meta-label");
    assert_eq!(label["flex"], "0 0 auto");
    assert_eq!(label["margin-bottom"], "0");
    assert_eq!(label["white-space"], "nowrap");
    assert_eq!(rule(".skill-badge strong")["min-width"], "0");
    assert_eq!(rule(".skill-badge strong")["overflow-wrap"], "anywhere");
    let tree = include_str!("../components/route_tree.rs");
    let group = tree
        .split_once("<div class=\"skill-source-group skill-badges\">")
        .unwrap()
        .1
        .split_once("</div>")
        .unwrap()
        .0;
    assert!(group.contains("<span class=\"meta-label\">"));
    assert!(group.contains("source == Message::OwnSkills"));
    assert!(group.contains("skill_learning(id, &upgrade_skills)"));
    assert!(group.contains("skill_badge(state, id, learning)"));
    assert_eq!(rule(".skill-learning")["flex"], "0 0 auto");
    assert_eq!(rule(".skill-learning")["white-space"], "nowrap");
    assert_eq!(rule(".node-topline, .option-titleline")["display"], "flex");
    assert_eq!(rule(".option-titleline")["flex-wrap"], "wrap");
    assert_eq!(rule(".route-metric")["overflow-wrap"], "anywhere");
    assert_eq!(
        rule(".route-metric strong")["font-variant-numeric"],
        "tabular-nums"
    );
    let source = include_str!("../components/source_picker.rs");
    assert_eq!(source.matches("i18n.text(Message::Macca)").count(), 1);
    assert_eq!(source.matches("i18n.text(Message::Score)").count(), 1);
    assert_eq!(
        source.matches("<div class=\"option-titleline\">").count(),
        2
    );
    for title in source.split("<div class=\"option-titleline\">").skip(1) {
        assert!(title.split_once("</div>").unwrap().0.contains("{metrics}"));
    }
}

#[test]
fn material_names_align_at_the_top_independently_of_skill_rows() {
    assert_eq!(rule(".material-entry")["align-items"], "stretch");
    assert_eq!(rule(".material-row")["align-items"], "flex-start");
    assert_eq!(rule(".material-row > div")["align-items"], "baseline");
    assert_eq!(
        rule(".skill-option strong, .material-row strong")["white-space"],
        "nowrap"
    );
    let mobile = css_block(CSS, "@media (max-width: 480px)");
    let details = declarations(css_block(mobile, ".material-row > div"));
    assert_eq!(details["grid-template-columns"], "minmax(0, 1fr)");
}

#[test]
fn card_metrics_align_right_and_node_levels_stay_inline() {
    let metrics = rule(".route-metrics");
    assert_eq!(metrics["display"], "flex");
    assert_eq!(metrics["flex-wrap"], "wrap");
    assert_eq!(metrics["justify-content"], "flex-end");
    assert_eq!(metrics["margin-left"], "auto");
    assert_eq!(metrics["text-align"], "right");
    assert_eq!(metrics["min-width"], "0");
    let tree = include_str!("../components/route_tree.rs");
    let method = tree
        .split_once("<div class=\"node-method\">")
        .unwrap()
        .1
        .split_once("</div>")
        .unwrap()
        .0;
    assert!(method.contains("<LevelFlow initial_level=base_level final_level />"));
    assert!(method.contains("class=\"route-metrics\""));
    assert!(method.contains("i18n.text(Message::Macca)"));
    let options = include_str!("../components/source_picker.rs");
    assert!(options.contains("<span class=\"route-metrics\">"));
}

#[test]
fn result_notices_stack_below_the_route_count_and_actions() {
    let summary = RESULT_PANEL
        .split_once("<div class=\"result-summary card\">")
        .unwrap()
        .1
        .split_once("{match tree")
        .unwrap()
        .0;
    assert!(
        summary.find("result-summary-layout").unwrap() < summary.find("result-notices").unwrap()
    );
    assert_eq!(
        summary.matches("class=\"button button-secondary\"").count(),
        3
    );
    assert_eq!(summary.matches("class=\"result-notice\"").count(), 3);
    let stale = summary.find("Message::StaleResult").unwrap();
    let read_only = summary.find("Message::ResultReadOnly").unwrap();
    let element_price = summary.find("Message::ElementPriceNote").unwrap();
    assert!(stale < read_only && read_only < element_price);
    assert!(!summary.contains("Message::ActualDepth"));
    assert!(!summary.contains("Message::RouteCountHelp"));

    let notices = rule(".result-notices");
    assert_eq!(notices["display"], "grid");
    assert_eq!(notices["gap"], "0.5rem");
    assert_eq!(notices["margin-top"], "0.85rem");
    assert!(!notices.contains_key("position"));

    let notice = rule(".result-notice");
    assert_eq!(notice["margin"], "0");
    assert_eq!(notice["border"], "1px solid var(--gold)");
    assert_eq!(notice["border-radius"], "var(--radius-small)");
    assert_eq!(notice["background"], "var(--gold-soft)");
}

#[test]
fn touch_and_narrow_sidebars_grow_with_content_instead_of_scrolling_internally() {
    use super::super::events::TOUCH_OR_NO_HOVER_QUERY;

    let desktop = rule(".search-panel");
    assert_eq!(desktop["position"], "sticky");
    assert_eq!(desktop["max-height"], "calc(100vh - 2rem)");
    assert_eq!(desktop["overflow-y"], "auto");
    assert_eq!(
        TOUCH_OR_NO_HOVER_QUERY,
        "(any-pointer: coarse), (hover: none)"
    );
    for query in [TOUCH_OR_NO_HOVER_QUERY, "(max-width: 880px)"] {
        let media = css_block(CSS, &format!("@media {query}"));
        let panel = declarations(css_block(media, ".search-panel"));
        assert_eq!(panel["position"], "static");
        assert_eq!(panel["max-height"], "none");
        assert_eq!(panel["overflow"], "visible");
        assert_eq!(panel["scrollbar-gutter"], "auto");
    }
}

#[test]
fn project_links_form_a_plain_footer_below_the_workspace() {
    let footer_start = PAGE.find("<footer class=\"site-footer\"").unwrap();
    let footer_end = PAGE[footer_start..].find("</footer>").unwrap() + footer_start;
    let footer = &PAGE[footer_start..footer_end];
    assert!(PAGE.find("</main>").unwrap() < footer_start);
    assert!(footer_end < PAGE.find("<SkillPicker />").unwrap());
    assert!(footer.contains("inert=move || state.modal_open()"));
    assert!(footer.contains("aria-label=move || i18n.text(Message::ProjectLinks)"));
    assert!(
        footer.find("Message::SourceRepository").unwrap()
            < footer.find("Message::SubmitFeedback").unwrap()
    );
    assert_eq!(footer.matches("target=\"_blank\"").count(), 2);
    assert_eq!(footer.matches("rel=\"noopener noreferrer\"").count(), 2);
    assert!(PAGE.contains(
        "const SOURCE_REPOSITORY_URL: &str = \"https://github.com/raspirin/smt5-fusion\";"
    ));
    assert!(PAGE.contains(
        "const FEEDBACK_URL: &str = \"https://github.com/raspirin/smt5-fusion/issues/new\";"
    ));

    let shell = rule(".app-shell");
    assert_eq!(shell["display"], "flex");
    assert_eq!(shell["flex-direction"], "column");
    assert_eq!(rule(".workspace")["flex"], "1 0 auto");
    let footer_style = rule(".site-footer");
    for property in ["border", "border-radius", "background", "box-shadow"] {
        assert!(!footer_style.contains_key(property));
    }
    let links = rule(".footer-links");
    assert_eq!(links["display"], "flex");
    assert_eq!(links["flex-wrap"], "wrap");
    assert_eq!(links["justify-content"], "center");
    assert_eq!(links["font-size"], "0.84rem");
    assert_eq!(rule(".footer-links a")["min-height"], "44px");
}

#[test]
fn preview_tape_replaces_the_header_rule_without_clipping_or_covering_controls() {
    let header = rule(".site-header");
    assert_eq!(header["border-bottom"], "4px solid var(--orange)");
    assert!(!header.contains_key("overflow"));
    assert!(!CSS.contains(".site-header.preview"));
    let tape = rule(".preview-tape");
    assert_eq!(
        tape["margin-top"],
        format!(
            "-{}",
            header["border-bottom"].split_whitespace().next().unwrap()
        )
    );
    assert_eq!(tape["display"], "block");
    assert_eq!(tape["width"], "100%");
    assert_eq!(tape["pointer-events"], "none");
    for property in ["position", "transform", "overflow"] {
        assert!(!tape.contains_key(property));
    }
    let input_height = rule(".text-input, .select-input")["height"]
        .trim_end_matches("px")
        .parse::<f64>()
        .unwrap();
    let tape_height = tape["height"]
        .trim_end_matches("px")
        .parse::<f64>()
        .unwrap();
    assert!((tape_height - input_height * 2.0 / 3.0).abs() < 1.0);
    let stripes = tape["background"];
    assert!(stripes.contains("repeating-linear-gradient("));
    assert!(stripes.contains("var(--preview-yellow) 0 15px"));
    assert!(stripes.contains("var(--preview-black) 16px 31px"));
    assert!(stripes.contains("var(--preview-yellow) 32px"));
    assert!(!CSS.contains(".preview-tape::before"));
    assert!(!PAGE.contains("class:preview"));
    let placement = PAGE.find("{preview_tape(i18n)}").unwrap();
    assert!(PAGE.find("</header>").unwrap() < placement);
    assert!(placement < PAGE.find("<main").unwrap());
    for dark in [false, true] {
        check_contrast("--preview-yellow", "--preview-black", 7.0, &palette(dark));
    }
    assert!(PAGE.contains("#[cfg(feature = \"preview\")]"));
    assert!(PAGE.contains("class=\"preview-tape\""));
    assert!(PAGE.contains("role=\"img\""));
    assert!(PAGE.contains("aria-label=move || i18n.text(Message::PreviewLabel)"));
    for removed in ["preview-build", "preview-label", "BUILD_ID"] {
        assert!(!PAGE.contains(removed));
    }
}

#[test]
fn theme_transitions_are_scoped_color_only_and_respect_reduced_motion() {
    let duration = rule(":root")["--theme-duration"]
        .strip_suffix("ms")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert_eq!(duration, 280);
    assert!(THEME.contains("state.sync_document(false)"));
    let motion = CSS.find("@media (prefers-reduced-motion: reduce)").unwrap();
    let scoped = &CSS[CSS.find(":root[data-theme-transition],").unwrap()..motion];
    for property in [
        "background-color",
        "color",
        "border-color",
        "outline-color",
        "box-shadow",
    ] {
        assert!(scoped.contains(&format!("{property} var(--theme-duration) ease-in-out")));
    }
    for property in ["opacity", "transform", "height", "width", "all"] {
        assert!(!scoped.contains(&format!("{property} var(--theme-duration)")));
    }
    assert!(CSS[motion..].contains("transition-duration: 0.01ms !important"));
    let cleanup = THEME
        .split("Duration::from_millis(")
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(cleanup >= duration + 50);
}

#[test]
fn gradient_colors_are_registered_for_interpolation() {
    for property in ["--canvas", "--canvas-mid", "--canvas-glow", "--header-glow"] {
        let registration = rule(&format!("@property {property}"));
        assert_eq!(registration["syntax"], "\"<color>\"");
        assert_eq!(registration["inherits"], "true");
        assert!(CSS.contains(&format!("{property} var(--theme-duration) ease-in-out")));
    }
}

#[test]
fn native_color_schemes_and_browser_toolbar_colors_match_the_palette() {
    for (dark, scheme) in [(false, "light"), (true, "dark")] {
        let palette = palette(dark);
        assert_eq!(palette["color-scheme"], scheme);
        let meta = SOURCE
            .split("<meta")
            .filter_map(|part| part.split_once('>').map(|(tag, _)| tag))
            .find(|tag| {
                tag.contains("name=\"theme-color\"")
                    && tag.contains(&format!("data-theme=\"{scheme}\""))
            })
            .unwrap();
        assert!(meta.contains(&format!("content=\"{}\"", value("--header", &palette))));
    }
}

#[test]
fn dark_text_notices_buttons_and_focus_colors_meet_contrast_targets() {
    let dark = palette(true);
    for surface in [
        "--canvas",
        "--canvas-mid",
        "--surface",
        "--surface-muted",
        "--surface-raised",
        "--input-bg",
        "--picker-bg",
        "--header-control-bg",
    ] {
        for text in ["--ink", "--ink-soft", "--ink-faint"] {
            check_contrast(text, surface, 4.5, &dark);
        }
        check_contrast("--focus", surface, 3.0, &dark);
        check_contrast("--line-strong", surface, 3.0, &dark);
    }
    for surface in [
        "--surface",
        "--surface-raised",
        "--orange-soft",
        "--green-soft",
    ] {
        check_contrast("--orange-dark", surface, 4.5, &dark);
        check_contrast("--ink-soft", surface, 4.5, &dark);
    }
    for text in ["--green", "--dlc-ink", "--special-ink"] {
        check_contrast(text, "--surface", 4.5, &dark);
        check_contrast(text, "--surface-raised", 4.5, &dark);
    }
    for background in ["--primary-bg", "--primary-hover"] {
        check_contrast("--primary-ink", background, 4.5, &dark);
    }
    for (foreground, background) in [
        ("--warning-ink", "--gold-soft"),
        ("--danger", "--danger-soft"),
        ("--danger-ink", "--danger-soft"),
        ("--header-ink", "--header"),
    ] {
        check_contrast(foreground, background, 4.5, &dark);
    }
    check_contrast("--focus", "--orange-soft", 3.0, &dark);
}

#[test]
fn skill_categories_remain_readable_on_their_tinted_badges() {
    let dark = palette(true);
    assert!(CSS.contains("background: color-mix(in srgb, var(--kind-color) 9%, var(--kind-bg));"));
    let base = rgb(value("--kind-bg", &dark));
    let kinds = dark
        .keys()
        .filter(|name| name.starts_with("--kind-") && **name != "--kind-bg")
        .collect::<Vec<_>>();
    assert_eq!(kinds.len(), 12);
    for name in kinds {
        let foreground = rgb(value(name, &dark));
        let background = [0, 1, 2].map(|index| foreground[index] * 0.09 + base[index] * 0.91);
        assert!(contrast(foreground, background) >= 4.5, "{name}");
        assert!(CSS.contains(&format!("--kind-color: var({name});")));
    }
}

#[test]
fn component_colors_use_defined_palette_variables() {
    let mut defined = BTreeSet::new();
    for (selector, body) in rules() {
        let declarations = declarations(body);
        defined.extend(
            declarations
                .keys()
                .filter(|name| name.starts_with("--"))
                .copied(),
        );
        if selector == ":root" || selector == ":root[data-theme=\"dark\"]" {
            continue;
        }
        for value in declarations.values() {
            for forbidden in ["#", "rgb(", "rgba(", "hsl(", "hsla("] {
                assert!(!value.contains(forbidden), "{selector}: {value}");
            }
            assert!(
                !value
                    .split_whitespace()
                    .any(|word| word == "white" || word == "black")
            );
        }
    }
    for reference in CSS.split("var(").skip(1) {
        let name = reference.split([',', ')']).next().unwrap().trim();
        assert!(defined.contains(name), "Undefined {name}");
    }
}
