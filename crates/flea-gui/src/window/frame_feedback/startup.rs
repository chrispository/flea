//! Passive first-frame markers: no benchmark scroll driver and no changes to scene readiness.
use super::{policy, Submitted};
use crate::{startup_trace, state::Shell};
use gpui::{App, PresentationTime, Window};
use std::sync::atomic::{AtomicBool, Ordering};

static SUBMITTED: AtomicBool = AtomicBool::new(false);
static PRESENTED: AtomicBool = AtomicBool::new(false);
static LISTING: AtomicBool = AtomicBool::new(false);

pub(super) fn submitted() {
    if startup_trace::enabled() && !SUBMITTED.swap(true, Ordering::Relaxed) {
        startup_trace::mark("frame.first_submitted");
    }
}

pub(super) fn presented(
    submitted: &Submitted,
    time: PresentationTime,
    shell: &Shell,
    window: &Window,
    cx: &App,
) {
    if !startup_trace::enabled() || time.clock_id != Some(1) {
        return;
    }
    let Some(at) = time.timestamp else { return };
    if !PRESENTED.swap(true, Ordering::Relaxed) {
        startup_trace::at("frame.first_presented", at.as_nanos());
    }
    let current = submitted.visible
        && window.is_visible()
        && submitted.draw.epoch == shell.presentation.epoch;
    let panes = &submitted.draw.panes;
    let complete = panes.iter().any(Option::is_some)
        && panes.iter().enumerate().all(|(side, frame)| {
            let Some(frame) = frame else { return true };
            let list = frame.list.borrow();
            frame.ready
                && !frame.loading
                && !frame.blank
                && (frame.content.held.total == 0
                    || (!frame.content.painted_rows.is_empty()
                        && frame.content.painted_rows.len() == list.visible().len()))
                && shell.feedback_eligible(side, window, cx)
                && shell
                    .shown_pane(side)
                    .is_some_and(|host| policy::current(host, frame))
        });
    if current && complete && !LISTING.swap(true, Ordering::Relaxed) {
        startup_trace::at("frame.first_listing_presented", at.as_nanos());
    }
}
