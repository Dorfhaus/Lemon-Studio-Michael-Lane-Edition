// SPDX-License-Identifier: GPL-3.0-or-later
use super::spotlight;
use std::sync::OnceLock;

pub(super) struct Chapter {
    pub title: &'static str,
    pub intro: &'static str,
    pub topics: std::ops::Range<usize>,
}

pub(super) struct Topic {
    pub chapter: usize,
    pub title: &'static str,
    pub keywords: &'static str,
    pub blocks: Vec<Block>,
    pub picture: Option<&'static str>,
    pub related: Vec<&'static str>,
    pub destination: Option<spotlight::Target>,
    index: String,
}

pub(super) enum Block {
    Paragraph(&'static str),
    Heading(&'static str),
    ListItem(&'static str, bool),
    MenuReference(Option<usize>),
}

#[derive(Default)]
struct Guide {
    chapters: Vec<Chapter>,
    topics: Vec<Topic>,
}

fn guide() -> &'static Guide {
    static GUIDE: OnceLock<Guide> = OnceLock::new();
    GUIDE.get_or_init(|| {
        let mut guide = Guide::default();
        for line in include_str!("../../../../docs/user-guide.md")
            .lines()
            .map(str::trim)
        {
            if let Some(title) = line.strip_prefix("## ") {
                let start = guide.topics.len();
                guide.chapters.push(Chapter {
                    title,
                    intro: "",
                    topics: start..start,
                });
                continue;
            }
            let Some(chapter) = guide.chapters.last_mut() else {
                continue;
            };
            if let Some(intro) = line.strip_prefix("Overview: ") {
                chapter.intro = intro;
                continue;
            }
            if let Some(title) = line.strip_prefix("### ") {
                chapter.topics.end += 1;
                guide.topics.push(Topic {
                    chapter: guide.chapters.len() - 1,
                    title,
                    keywords: "",
                    blocks: Vec::new(),
                    picture: None,
                    related: Vec::new(),
                    destination: None,
                    index: String::new(),
                });
                continue;
            }
            let Some(topic) = guide.topics.last_mut() else {
                continue;
            };
            if line.is_empty() {
                continue;
            }
            if let Some(words) = line.strip_prefix("Keywords: ") {
                topic.keywords = words;
            } else if let Some(target) = line.strip_prefix("Guide: ") {
                topic.destination =
                    Some(spotlight::Target::from_key(target).expect("bundled guide destination"));
            } else if let Some(alt) = line
                .strip_prefix("![")
                .and_then(|line| line.split_once("]("))
                .map(|(alt, _)| alt)
            {
                topic.picture = Some(alt);
            } else if let Some(titles) = line.strip_prefix("Related: ") {
                topic.related = titles.split(" | ").collect();
            } else if let Some(title) = line.strip_prefix("#### ") {
                topic.blocks.push(Block::Heading(title));
            } else if let Some(item) = line.strip_prefix("- ") {
                topic.blocks.push(Block::ListItem(item, false));
            } else if line
                .split_once(". ")
                .is_some_and(|(prefix, _)| prefix.parse::<u32>().is_ok())
            {
                topic.blocks.push(Block::ListItem(line, true));
            } else if let Some(menu) = line
                .strip_prefix("<!-- menu-reference")
                .and_then(|line| line.strip_suffix("-->"))
            {
                let menu = menu.trim();
                let index = (!menu.is_empty()).then(|| {
                    crate::views::menus::VINTAGE_TITLES
                        .iter()
                        .position(|title| *title == menu)
                        .expect("bundled guide menu")
                });
                topic.blocks.push(Block::MenuReference(index));
            } else {
                topic.blocks.push(Block::Paragraph(line));
            }
        }
        for topic in &mut guide.topics {
            let mut index = format!(
                "{} {} {}",
                guide.chapters[topic.chapter].title, topic.title, topic.keywords
            );
            for block in &topic.blocks {
                index.push(' ');
                match block {
                    Block::Paragraph(text) | Block::Heading(text) | Block::ListItem(text, _) => {
                        index.push_str(text)
                    }
                    Block::MenuReference(menu) => {
                        for (_, _, items) in menu_groups()
                            .filter(|(index, ..)| menu.is_none_or(|menu| menu == *index))
                        {
                            for item in items {
                                index.push_str(item.label);
                                index.push(' ');
                                index.push_str(item.shortcut);
                                index.push(' ');
                            }
                        }
                    }
                }
            }
            topic.index = index.to_lowercase();
        }
        guide
    })
}

pub(super) fn menu_groups() -> impl Iterator<
    Item = (
        usize,
        &'static str,
        impl Iterator<Item = &'static crate::views::menus::Item>,
    ),
> {
    use crate::views::menus::{self, Kind};
    menus::MENUS.iter().enumerate().map(|(index, menu)| {
        (
            index,
            menus::VINTAGE_TITLES[index],
            menu.cols
                .iter()
                .flat_map(|(_, items)| *items)
                .filter(move |item| item.kind == Kind::Item && menus::works(index, item)),
        )
    })
}

pub(super) fn chapters() -> &'static [Chapter] {
    &guide().chapters
}
pub(super) fn topics() -> &'static [Topic] {
    &guide().topics
}

pub(super) fn results(query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    let terms: Vec<_> = query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| {
            !word.is_empty()
                && !matches!(
                    *word,
                    "how" | "do" | "i" | "can" | "to" | "the" | "a" | "an" | "my" | "in" | "and"
                )
        })
        .collect();
    if terms.is_empty() {
        return Vec::new();
    }
    let mut matches: Vec<_> = topics()
        .iter()
        .enumerate()
        .filter_map(|(index, topic)| {
            if !terms.iter().all(|term| topic.index.contains(term)) {
                return None;
            }
            let title = topic.title.to_lowercase();
            let keywords = topic.keywords.to_lowercase();
            let rank: u32 = terms
                .iter()
                .map(|term| {
                    if title.contains(term) {
                        5
                    } else if keywords.contains(term) {
                        3
                    } else {
                        0
                    }
                })
                .sum();
            Some((index, rank))
        })
        .collect();
    matches.sort_by_key(|&(index, rank)| (std::cmp::Reverse(rank), index));
    matches.into_iter().map(|(index, _)| index).collect()
}
