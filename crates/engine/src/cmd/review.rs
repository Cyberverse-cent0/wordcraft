//! Review tab: comments, track changes, word count, spelling (via the proofing hook).

use serde_json::{Value, json};
use wordcraft_doc::para::InlineObject;
use wordcraft_doc::{Comment, Paragraph, PartKind, Pos, StoryRef, para_block};

use super::{now_iso, pos_json, sel_result};
use crate::{CmdError, CmdResult, CommandSpec, Selection, Session, p};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("review.newComment", "New Comment", "Review › Comments", new_comment).key("Mod+Alt+M").params(r#"{"text": string}"#),
        CommandSpec::new("review.reply", "Reply", "Review › Comments", reply).params(r#"{"id": n, "text": string}"#),
        CommandSpec::new("review.deleteComment", "Delete", "Review › Comments", delete_comment).params(r#"{"id"?: n, "all"?: bool}"#),
        CommandSpec::new("review.resolveComment", "Resolve", "Review › Comments", |s, v| {
            let id = p::u64(v, "id").map(|x| x as u32).or_else(|| comment_at_caret(s)).ok_or_else(|| CmdError::Params("`id` required".into()))?;
            let c = s.doc.comments.get_mut(&id).ok_or_else(|| CmdError::Params("no such comment".into()))?;
            c.resolved = p::bool(v, "value").unwrap_or(!c.resolved);
            sel_result(s)
        }),
        CommandSpec::new("review.nextComment", "Next", "Review › Comments", |s, _| nav_comment(s, 1)).pure(),
        CommandSpec::new("review.previousComment", "Previous", "Review › Comments", |s, _| nav_comment(s, -1)).pure(),
        CommandSpec::new("review.comments", "Show Comments", "Review › Comments", list_comments).pure(),
        CommandSpec::new("review.trackChanges", "Track Changes", "Review › Tracking", |s, v| {
            s.doc.settings.track_changes = p::bool(v, "value").unwrap_or(!s.doc.settings.track_changes);
            Ok(json!({"value": s.doc.settings.track_changes}))
        })
        .key("Mod+Shift+E"),
        CommandSpec::new("review.acceptAll", "Accept All Changes", "Review › Changes", |s, _| resolve_all(s, true)),
        CommandSpec::new("review.rejectAll", "Reject All Changes", "Review › Changes", |s, _| resolve_all(s, false)),
        CommandSpec::new("review.accept", "Accept", "Review › Changes", |s, _| resolve_sel(s, true)),
        CommandSpec::new("review.reject", "Reject", "Review › Changes", |s, _| resolve_sel(s, false)),
        CommandSpec::new("review.nextChange", "Next Change", "Review › Changes", |s, _| nav_change(s, 1)).pure(),
        CommandSpec::new("review.previousChange", "Previous Change", "Review › Changes", |s, _| nav_change(s, -1)).pure(),
        CommandSpec::new("review.markup", "Display for Review", "Review › Tracking", |s, v| {
            s.view.show_markup = p::str(v, "value").map(|m| m != "noMarkup" && m != "original").unwrap_or(!s.view.show_markup);
            Ok(json!({"showMarkup": s.view.show_markup}))
        })
        .pure(),
        CommandSpec::new("review.wordCount", "Word Count", "Review › Proofing", word_count).pure(),
        CommandSpec::new("review.changes", "Reviewing Pane", "Review › Tracking", list_changes).pure(),
    ]
}

fn new_comment(s: &mut Session, v: &Value) -> CmdResult {
    let text = p::str(v, "text").unwrap_or("").to_string();
    let (a, b) = s.sel.ordered();
    let (a, b) = if a == b {
        // Comment on the word at the caret.
        match s.doc.para_at(&a) {
            Some(para) if !para.is_empty() => {
                let (x, y) = para.word_at(a.off);
                let y = para.text.get(x..y).map(|w| x + w.trim_end().len()).unwrap_or(y);
                (Pos { off: x, ..a.clone() }, Pos { off: y, ..a })
            }
            _ => (a.clone(), a),
        }
    } else {
        (a, b)
    };
    let part = s.doc.add_part(PartKind::Comment, vec![para_block(Paragraph::with_text(&text, Default::default()))]);
    let id = s.doc.comments.keys().next_back().map(|k| k + 1).unwrap_or(0);
    let initials: String = s.author.split_whitespace().filter_map(|w| w.chars().next()).collect();
    s.doc.comments.insert(id, Comment { author: s.author.clone(), initials, date: now_iso(), parent: None, resolved: false, part });
    let props = Default::default();
    s.doc.insert_object(&b, InlineObject::CommentEnd { id }, &props)?;
    s.doc.insert_object(&a, InlineObject::CommentStart { id }, &props)?;
    s.view.comments_pane = true;
    Ok(json!({"id": id, "story": part}))
}

fn reply(s: &mut Session, v: &Value) -> CmdResult {
    let parent = p::u64(v, "id").map(|x| x as u32).ok_or_else(|| CmdError::Params("`id` required".into()))?;
    if !s.doc.comments.contains_key(&parent) {
        return Err(CmdError::Params("no such comment".into()));
    }
    let text = p::req_str(v, "text")?;
    let part = s.doc.add_part(PartKind::Comment, vec![para_block(Paragraph::with_text(text, Default::default()))]);
    let id = s.doc.comments.keys().next_back().map(|k| k + 1).unwrap_or(0);
    let initials: String = s.author.split_whitespace().filter_map(|w| w.chars().next()).collect();
    s.doc.comments.insert(id, Comment { author: s.author.clone(), initials, date: now_iso(), parent: Some(parent), resolved: false, part });
    Ok(json!({"id": id}))
}

/// Comment anchored around the caret.
fn comment_at_caret(s: &Session) -> Option<u32> {
    let f = &s.sel.focus;
    let p = s.doc.para_at(f)?;
    let mut open: Vec<u32> = Vec::new();
    for off in p.object_offsets() {
        if off > f.off {
            break;
        }
        match p.object_at(off) {
            Some(InlineObject::CommentStart { id }) => open.push(*id),
            Some(InlineObject::CommentEnd { id }) => open.retain(|x| x != id),
            _ => {}
        }
    }
    open.last().copied().or_else(|| {
        p.object_offsets().into_iter().find_map(|o| match p.object_at(o) {
            Some(InlineObject::CommentStart { id }) | Some(InlineObject::CommentEnd { id }) => Some(*id),
            _ => None,
        })
    })
}

fn remove_anchors(s: &mut Session, ids: &[u32]) -> Result<(), CmdError> {
    for story in [StoryRef::Body] {
        for path in s.doc.para_paths(story) {
            let offs: Vec<usize> = s
                .doc
                .para(story, &path)
                .map(|p| {
                    p.object_offsets()
                        .into_iter()
                        .filter(|o| matches!(p.object_at(*o), Some(InlineObject::CommentStart { id }) | Some(InlineObject::CommentEnd { id }) if ids.contains(id)))
                        .collect()
                })
                .unwrap_or_default();
            if offs.is_empty() {
                continue;
            }
            let para = s.doc.para_mut(story, &path)?;
            for o in offs.into_iter().rev() {
                para.delete(o, o + wordcraft_doc::para::OBJ.len_utf8())?;
            }
        }
    }
    Ok(())
}

fn delete_comment(s: &mut Session, v: &Value) -> CmdResult {
    let ids: Vec<u32> = if p::bool(v, "all").unwrap_or(false) {
        s.doc.comments.keys().copied().collect()
    } else {
        let id = p::u64(v, "id").map(|x| x as u32).or_else(|| comment_at_caret(s)).ok_or_else(|| CmdError::Params("no comment here".into()))?;
        // The comment and its replies.
        let mut v = vec![id];
        v.extend(s.doc.comments.iter().filter(|(_, c)| c.parent == Some(id)).map(|(k, _)| *k));
        v
    };
    remove_anchors(s, &ids)?;
    for id in &ids {
        if let Some(c) = s.doc.comments.remove(id) {
            s.doc.parts.remove(&c.part);
        }
    }
    s.clamp_selection();
    Ok(json!({"deleted": ids}))
}

/// Comments in document order with their anchors.
pub fn comment_list(s: &Session) -> Vec<(u32, Option<Pos>)> {
    let mut found: Vec<(u32, Option<Pos>)> = Vec::new();
    for path in s.doc.para_paths(StoryRef::Body) {
        let Some(p) = s.doc.para(StoryRef::Body, &path) else { continue };
        for o in p.object_offsets() {
            if let Some(InlineObject::CommentStart { id }) = p.object_at(o) {
                found.push((*id, Some(Pos { story: StoryRef::Body, path: path.clone(), off: o })));
            }
        }
    }
    for id in s.doc.comments.keys() {
        if !found.iter().any(|(x, _)| x == id) {
            found.push((*id, None));
        }
    }
    found
}

fn list_comments(s: &mut Session, _: &Value) -> CmdResult {
    let list = comment_list(s);
    Ok(Value::Array(
        list.iter()
            .filter_map(|(id, pos)| {
                let c = s.doc.comments.get(id)?;
                Some(json!({
                    "id": id, "author": c.author, "date": c.date, "resolved": c.resolved, "parent": c.parent,
                    "text": s.doc.plain_text(StoryRef::Part(c.part)),
                    "anchor": pos.as_ref().map(pos_json),
                }))
            })
            .collect(),
    ))
}

fn nav_comment(s: &mut Session, dir: i32) -> CmdResult {
    let list: Vec<Pos> = comment_list(s).into_iter().filter_map(|(_, p)| p).collect();
    let caret = s.sel.focus.clone();
    let t = if dir > 0 { list.iter().find(|p| **p > caret).or(list.first()) } else { list.iter().rev().find(|p| **p < caret).or(list.last()) };
    if let Some(p) = t {
        s.sel = Selection::caret(p.clone());
    }
    sel_result(s)
}

/// Accept or reject a revision range in one paragraph.
fn resolve_para(s: &mut Session, story: StoryRef, path: &wordcraft_doc::Path, from: usize, to: usize, accept: bool) -> Result<(), CmdError> {
    let para = s.doc.para_mut(story, path)?;
    let ranges: Vec<(usize, usize, bool, bool)> =
        para.run_ranges().filter(|(r, _)| r.end > from && r.start < to).map(|(r, c)| (r.start.max(from), r.end.min(to), c.ins.is_some(), c.del.is_some())).collect();
    for (a, b, ins, del) in ranges.into_iter().rev() {
        if (del && accept) || (ins && !accept) {
            para.delete(a, b)?;
        } else if ins || del {
            para.format(a, b, &|c| {
                c.ins = None;
                c.del = None;
            })?;
        }
    }
    if para.mark.ins.is_some() {
        para.mark.ins = None;
    }
    Ok(())
}

fn resolve_all(s: &mut Session, accept: bool) -> CmdResult {
    let stories: Vec<StoryRef> = std::iter::once(StoryRef::Body).chain(s.doc.parts.keys().map(|k| StoryRef::Part(*k))).collect();
    for st in stories {
        for path in s.doc.para_paths(st).into_iter().rev() {
            let len = s.doc.para(st, &path).map(|p| p.len()).unwrap_or(0);
            resolve_para(s, st, &path, 0, len, accept)?;
        }
    }
    s.doc.revisions.clear();
    s.clamp_selection();
    sel_result(s)
}

fn resolve_sel(s: &mut Session, accept: bool) -> CmdResult {
    let (a, b) = s.sel.ordered();
    let (a, b) = if a == b {
        // The change at the caret: the run around it.
        let found = s.doc.para_at(&a).and_then(|p| p.run_ranges().find(|(r, c)| r.start <= a.off && a.off <= r.end && (c.ins.is_some() || c.del.is_some())).map(|(r, _)| r));
        match found {
            Some(r) => (Pos { off: r.start, ..a.clone() }, Pos { off: r.end, ..a }),
            None => return nav_change(s, 1),
        }
    } else {
        (a, b)
    };
    for path in s.doc.paths_between(&a, &b).into_iter().rev() {
        let len = s.doc.para(a.story, &path).map(|p| p.len()).unwrap_or(0);
        let from = if path == a.path { a.off } else { 0 };
        let to = if path == b.path { b.off } else { len };
        resolve_para(s, a.story, &path, from, to, accept)?;
    }
    s.sel = Selection::caret(a);
    s.clamp_selection();
    sel_result(s)
}

fn changes(s: &Session) -> Vec<(Pos, Pos, &'static str, Option<u32>)> {
    let mut out = Vec::new();
    for path in s.doc.para_paths(StoryRef::Body) {
        let Some(p) = s.doc.para(StoryRef::Body, &path) else { continue };
        for (r, c) in p.run_ranges() {
            let kind = if c.ins.is_some() {
                "insert"
            } else if c.del.is_some() {
                "delete"
            } else {
                continue;
            };
            let mk = |off| Pos { story: StoryRef::Body, path: path.clone(), off };
            out.push((mk(r.start), mk(r.end), kind, c.ins.or(c.del)));
        }
    }
    out
}

fn nav_change(s: &mut Session, dir: i32) -> CmdResult {
    let list = changes(s);
    let caret = s.sel.ordered();
    let t = if dir > 0 { list.iter().find(|c| c.0 >= caret.1).or(list.first()) } else { list.iter().rev().find(|c| c.1 <= caret.0).or(list.last()) };
    if let Some((a, b, _, _)) = t {
        s.sel = Selection { anchor: a.clone(), focus: b.clone() };
    }
    sel_result(s)
}

fn list_changes(s: &mut Session, _: &Value) -> CmdResult {
    let list = changes(s);
    Ok(Value::Array(
        list.iter()
            .map(|(a, b, kind, rid)| {
                let rev = rid.and_then(|r| s.doc.revisions.get(r as usize));
                json!({
                    "kind": kind, "start": pos_json(a), "end": pos_json(b),
                    "text": s.doc.copy_range(a, b).plain_text(),
                    "author": rev.map(|r| r.author.clone()), "date": rev.map(|r| r.date.clone()),
                })
            })
            .collect(),
    ))
}

fn word_count(s: &mut Session, _: &Value) -> CmdResult {
    let text = if s.sel.is_collapsed() { s.doc.plain_text(StoryRef::Body) } else { s.selected_text() };
    let words = wordcraft_doc::count_words(&text);
    let chars = text.chars().filter(|c| *c != '\n').count();
    let chars_no_spaces = text.chars().filter(|c| !c.is_whitespace()).count();
    let paragraphs = text.split('\n').filter(|l| !l.trim().is_empty()).count();
    let lines: usize = {
        let l = s.layout();
        l.pages.iter().flat_map(|p| p.items.iter()).map(|it| if let wordcraft_layout::Placed::Lines { l0, l1, story: StoryRef::Body, .. } = it { l1 - l0 } else { 0 }).sum()
    };
    let pages = s.layout().pages.len();
    Ok(json!({"pages": pages, "words": words, "characters": chars_no_spaces, "charactersWithSpaces": chars, "paragraphs": paragraphs, "lines": lines}))
}
