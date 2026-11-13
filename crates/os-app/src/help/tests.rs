// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::app::{Scroll, Win};
use crate::views::menus::{self, HELP_GUIDE, HELP_MENU};
use os_ui::Canvas;

fn click(app: &mut App, action: Hit) {
    let r = app
        .hits
        .iter()
        .find(|h| h.action == hit(action) && !h.rect.is_empty())
        .unwrap()
        .rect;
    app.mouse_move(r.cx(), r.cy());
    app.mouse_down();
    app.mouse_up();
}

fn shot(canvas: &Canvas, name: &str) {
    if let Some(dir) = std::env::var_os("OS_SHOT_DIR") {
        canvas
            .pixmap
            .save_png(std::path::PathBuf::from(dir).join(name))
            .unwrap();
    }
}

#[test]
fn guide_catalog_links_images_and_stock_plugins() {
    use super::content::Block;
    assert_eq!(chapters().len(), 13);
    assert!(topics().len() >= 140);
    for (i, chapter) in chapters().iter().enumerate() {
        assert!(!chapter.intro.is_empty() && !chapter.topics.is_empty());
        assert!(chapter.topics.clone().all(|t| topics()[t].chapter == i));
    }
    for topic in topics() {
        assert!(
            !topic.keywords.is_empty(),
            "missing keywords in {}",
            topic.title
        );
        assert!(topic.blocks.len() >= 2, "empty article {}", topic.title);
        for title in &topic.related {
            assert_eq!(
                topics()
                    .iter()
                    .filter(|topic| topic.title == *title)
                    .count(),
                1,
                "{title}"
            );
        }
        if let Some(name) = topic.picture {
            assert!(wiki::picture_for(name).is_some(), "{name}");
        }
    }
    let instruments = [
        "ML Sampler",
        "ML Trio",
        "ML Wave",
        "ML Plasma",
        "ML Chop",
        "ML Citrus",
    ];
    for name in instruments
        .into_iter()
        .chain(crate::plugins::LINKED_EFFECTS)
        .chain(crate::plugins::FILE_MAPPED)
        .chain([crate::master_assistant::NAME])
    {
        assert!(
            topics()
                .iter()
                .any(|topic| matches!(topic.chapter, 6 | 7) && topic.title.contains(name)),
            "{name}"
        );
    }
    assert!(topics().iter().any(|topic| {
        topic
            .blocks
            .iter()
            .any(|b| matches!(b, Block::MenuReference(_)))
    }));
    assert_eq!(
        topics()[results("How do I warp audio?")[0]].title,
        "Edit audio timing with warp markers"
    );
    assert!(!results("ctrl wheel").is_empty());
    assert!(!results("hardware out").is_empty());
    assert!(results("zzzznoresult").is_empty());
}

#[test]
fn guide_menu_browse_search_read_picture_and_visibility() {
    let mut app = App::new(None);
    let item = menus::item_at(HELP_MENU, HELP_GUIDE).unwrap();
    assert_eq!(item.label, "User Guide");
    assert!(menus::in_menu(&app.state, HELP_MENU, item));

    for (w, h) in [(1280, 800), (700, 460), (430, 320)] {
        app.state.guide = View::default();
        let mut canvas = Canvas::new(w, h, 1.).unwrap();
        app.menu_item(HELP_MENU, HELP_GUIDE);
        assert!(is_open(&app.state));
        app.paint(&mut canvas, None);
        shot(&canvas, &format!("guide-home-{w}.png"));
        click(&mut app, Hit::Section(0));
        assert_eq!(app.state.guide.page, Page::Chapter(0));
        app.paint(&mut canvas, None);
        click(&mut app, Hit::Topic(0));
        assert_eq!(app.state.guide.page, Page::Topic(0));
        app.paint(&mut canvas, None);
        if w < 800 {
            app.state.guide.scroll_by(140., body(&app.state).h);
            app.paint(&mut canvas, None);
        }
        click(&mut app, Hit::Picture);
        assert!(app.state.guide.picture);
        app.paint(&mut canvas, None);
        shot(&canvas, &format!("guide-picture-{w}.png"));
        app.key(&Key::Named(NamedKey::Escape), false, false);
        assert!(!app.state.guide.picture);
        app.key(&Key::Character("k".into()), true, false);
        app.key(&Key::Character("warp audio".into()), false, false);
        app.paint(&mut canvas, None);
        shot(&canvas, &format!("guide-search-{w}.png"));
        app.key(&Key::Named(NamedKey::Enter), false, false);
        let warp = results("warp audio")[0];
        assert_eq!(app.state.guide.topic(), Some(warp));
        app.paint(&mut canvas, None);
        shot(&canvas, &format!("guide-article-{w}.png"));
        app.key(&Key::Named(NamedKey::Escape), false, false);
        assert!(app.state.guide.searching());
        assert_eq!(app.state.guide.search.text.text, "warp audio");
        act(&mut app, Hit::Close);
    }
    crate::workspace_visibility::toggle(VISIBILITY);
    assert!(!menus::in_menu(&app.state, HELP_MENU, item));
    app.menu_item(HELP_MENU, HELP_GUIDE);
    assert!(!is_open(&app.state));
}

#[test]
fn detach_keeps_the_article_and_main_workspace_independent() {
    let mut app = App::new(None);
    let mut main = Canvas::new(1280, 800, 1.).unwrap();
    let mut detached = Canvas::new(1560, 1020, 1.5).unwrap();
    let topic = results("attach the mixer to the playlist")[0];
    open(&mut app);
    act(&mut app, Hit::Topic(topic));
    app.paint(&mut main, None);
    app.state.guide.scroll = 65.;
    let project = app.state.project.clone();
    let z = app.state.z.clone();
    click(&mut app, Hit::Detach);
    assert!(detached_open(&app.state));
    assert!(!is_open(&app.state));

    app.paint(&mut main, None);
    assert!(!app.hits.iter().any(|h| h.action == hit(Hit::Search)));
    assert!(app.hits.iter().any(|h| h.action == Action::Menu(0)));
    app.open_settings(settings::MIDI);
    assert!(!is_open(&app.state));
    assert!(detached_open(&app.state));

    let context = app.take_view_context();
    app.state.native_view = Some(Win::Guide);
    app.paint(&mut detached, None);
    assert!(app.bar(Scroll::Guide).is_some());
    assert!(app.hits.iter().any(|h| h.action == hit(Hit::Detach)));
    assert!(!app.hits.iter().any(|h| h.action == Action::Menu(0)));
    assert_eq!(app.state.guide.topic(), Some(topic));
    assert_eq!(app.state.guide.scroll, 65.);
    shot(&detached, "guide-detached-150.png");
    let height = detached_body_height(detached.size());
    scroll_detached(&mut app, 140., height);
    assert_eq!(app.state.guide.scroll, 205.);
    assert_eq!(app.state.settings.tab, settings::MIDI);
    assert_eq!(app.state.settings.scroll, 0.);
    app.restore_view_context(context);

    close_detached(&mut app);
    assert!(!detached_open(&app.state));
    open(&mut app);
    assert!(detached_open(&app.state));
    guide_act(&mut app, Hit::Detach, true);
    assert!(is_open(&app.state) && !detached_open(&app.state));
    assert_eq!(app.state.guide.topic(), Some(topic));
    assert_eq!(app.state.guide.scroll, 205.);
    assert!(app.state.project == project);
    assert_eq!(app.state.z, z);

    crate::workspace_visibility::toggle(DETACH_VISIBILITY);
    app.paint(&mut main, None);
    assert!(!app.hits.iter().any(|h| h.action == hit(Hit::Detach)));
    act(&mut app, Hit::Detach);
    assert!(!detached_open(&app.state));
}

#[test]
fn spotlight_reveals_control_and_returns_to_the_correct_window() {
    let mut app = App::new(None);
    app.state.project = crate::app::test_project();
    app.state.z.clear();
    let index = results("warp audio")[0];
    let mut canvas = Canvas::new(1280, 800, 1.).unwrap();
    let before = app.state.project.clone();
    for vintage in [false, true] {
        app.state.vintage_2005 = vintage;
        open(&mut app);
        act(&mut app, Hit::Topic(index));
        app.paint(&mut canvas, None);
        click(&mut app, Hit::ShowWhere);
        assert!(!is_open(&app.state));
        assert!(active(&app.state));
        assert_eq!(app.state.z.last(), Some(&Win::Playlist));
        app.paint(&mut canvas, None);
        let anchor = app
            .hits
            .iter()
            .find(|h| h.action == Action::Warp(crate::warp::Command::Toggle(0)))
            .unwrap()
            .rect;
        let x = anchor.cx().round() as u32;
        let y = (anchor.y - 3.).round() as u32;
        assert!((y.saturating_sub(1)..=y + 1).any(|y| {
            canvas
                .pixmap
                .pixel(x, y)
                .is_some_and(|p| (p.red(), p.green(), p.blue()) == (32, 150, 243))
        }));
        click(&mut app, Hit::ReturnToGuide);
        assert!(is_open(&app.state) && !active(&app.state));
        assert_eq!(app.state.guide.topic(), Some(index));
        assert!(app.state.project == before);
    }

    act(&mut app, Hit::Detach);
    let scan = results("find installed plugins")[0];
    app.state.native_view = Some(Win::Guide);
    guide_act(&mut app, Hit::Topic(scan), true);
    guide_act(&mut app, Hit::ShowWhere, true);
    app.state.native_view = None;
    assert_eq!(app.state.settings.tab, settings::PLUGINS);
    assert!(active(&app.state) && detached_open(&app.state));
    app.paint(&mut canvas, None);
    click(&mut app, Hit::ReturnToGuide);
    assert!(app.state.guide.focus_window && detached_open(&app.state));
    assert_eq!(app.state.guide.topic(), Some(scan));
    assert!(!active(&app.state));
}
