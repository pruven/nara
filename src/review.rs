use std::process::Command;

use gpui::{
    App, Bounds, ClipboardItem, Context, FocusHandle, KeyBinding, MouseButton, TitlebarOptions,
    Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};
use url::Url;

use crate::{
    BACKGROUND, BORDER, FOREGROUND, MUTED, SURFACE, browser,
    check::{self, Check},
};

actions!(
    nara,
    [
        Dismiss,
        PasteLink,
        CopyLink,
        CheckLink,
        UnshortenLink,
        RemoveTracking,
        UndoLinkChange,
        OpenLink,
        Settings
    ]
);

enum Status {
    Ready,
    Working(&'static str),
    Checked(Check),
    Note(String),
    Failed(String),
}

struct Review {
    url: Option<Url>,
    original: Option<Url>,
    status: Status,
    error: Option<String>,
    copied: bool,
    request: u64,
    focus: FocusHandle,
}

impl Review {
    fn set_link(&mut self, input: &str) {
        self.original = None;
        match check::parse(input) {
            Ok(url) => self.replace_link(Some(url)),
            Err(error) => {
                self.replace_link(None);
                self.error = Some(error.to_owned());
            }
        }
    }

    fn replace_link(&mut self, url: Option<Url>) {
        self.request = self.request.wrapping_add(1);
        self.status = Status::Ready;
        self.copied = false;
        self.error = None;
        self.url = url;
    }

    fn transform_link(&mut self, url: Url) {
        if self.original.is_none() {
            self.original = self.url.take();
        }
        self.replace_link(Some(url));
    }

    fn undo(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = self.original.take() {
            self.replace_link(Some(url));
            self.status = Status::Note("Original link restored.".to_owned());
            cx.notify();
        }
    }

    fn copy(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = &self.url {
            cx.write_to_clipboard(ClipboardItem::new_string(url.as_str().to_owned()));
            self.copied = true;
            cx.notify();
        }
    }

    fn paste(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.set_link(&text);
        } else {
            self.error = Some("Copy a web link to your clipboard first.".to_owned());
        }
        cx.notify();
    }

    fn check(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.request_link(false, cx);
    }

    fn unshorten(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.request_link(true, cx);
    }

    fn remove_tracking(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(url) = &self.url else { return };
        if matches!(self.status, Status::Working(_)) {
            return;
        }
        let Some((url, count)) = check::remove_tracking(url) else {
            return;
        };
        self.transform_link(url);
        let label = if count == 1 {
            "parameter"
        } else {
            "parameters"
        };
        self.status = Status::Note(format!("Removed {count} tracking {label}"));
        cx.notify();
    }

    fn request_link(&mut self, unshorten: bool, cx: &mut Context<Self>) {
        let Some(url) = self.url.clone() else { return };
        if matches!(self.status, Status::Working(_)) {
            return;
        }
        self.status = Status::Working(if unshorten {
            "Resolving redirects…"
        } else {
            "Checking HTTP response…"
        });
        self.error = None;
        let request = self.request;
        let destination = url.clone();
        let task = cx.background_executor().spawn(async move {
            if unshorten {
                check::unshorten(url).map(|(url, count)| {
                    (
                        Some(url),
                        Status::Note(if count == 0 {
                            "No redirects found.".to_owned()
                        } else {
                            format!(
                                "Expanded link through {count} {}",
                                if count == 1 { "redirect" } else { "redirects" }
                            )
                        }),
                    )
                })
            } else {
                check::check(url).map(|result| {
                    let redirected = (result.redirects > 0).then(|| result.destination.clone());
                    (redirected, Status::Checked(result))
                })
            }
        });
        cx.spawn(async move |review, cx| {
            let result = task.await;
            let _ = review.update(cx, |review, cx| {
                if review.request == request {
                    review.status = match result {
                        Ok((url, status)) => {
                            if let Some(url) = url.filter(|url| *url != destination) {
                                review.transform_link(url);
                            }
                            status
                        }
                        Err(error) => Status::Failed(error),
                    };
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(url) = &self.url else { return };
        match browser::preferred() {
            Some(browser) => match browser.open(url) {
                Ok(()) => window.remove_window(),
                Err(error) => self.error = Some(format!("Could not open the browser. {error}")),
            },
            None => self.settings(window, cx),
        }
        cx.notify();
    }

    fn settings(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) =
            std::env::current_exe().and_then(|exe| Command::new(exe).arg("--settings").spawn())
        {
            self.error = Some(format!("Could not open settings. {error}"));
            cx.notify();
        }
    }

    fn button(
        cx: &mut Context<Self>,
        name: String,
        primary: bool,
        action: fn(&mut Self, &mut Window, &mut Context<Self>),
    ) -> impl IntoElement {
        let background = if primary { 0xe8e5df } else { SURFACE };
        let foreground = if primary { BACKGROUND } else { FOREGROUND };
        div()
            .rounded(px(8.0))
            .border_1()
            .border_color(rgb(if primary { background } else { BORDER }))
            .bg(rgb(background))
            .text_color(rgb(foreground))
            .hover(|style| style.bg(rgb(if primary { 0xffffff } else { 0x343438 })))
            .cursor_pointer()
            .px_3()
            .py_2()
            .child(name)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _, window, cx| {
                    action(view, window, cx);
                }),
            )
    }
}

impl Render for Review {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let destination = self
            .url
            .as_ref()
            .map(|url| {
                let host = url.host_str().unwrap_or_default();
                let authority = url
                    .port()
                    .map_or_else(|| host.to_owned(), |port| format!("{host}:{port}"));
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().truncate().child(authority))
                    .child(
                        div()
                            .text_sm()
                            .truncate()
                            .text_color(rgb(MUTED))
                            .child(url.as_str().to_owned()),
                    )
            })
            .unwrap_or_else(|| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child("Open a link with Nara"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("Or paste a web link from the clipboard."),
                    )
            });

        let result = match &self.status {
            Status::Ready => div().text_color(rgb(MUTED)).child(if self.url.is_some() {
                "Check and Unshorten contact the site."
            } else {
                "Ctrl+V to paste a link"
            }),
            Status::Working(message) => div().text_color(rgb(MUTED)).child(*message),
            Status::Note(message) => div().text_color(rgb(MUTED)).child(message.clone()),
            Status::Checked(check) => {
                let mut result = div().flex().flex_col().gap_1().child(format!(
                    "{}  ·  {} ms",
                    check.status,
                    check.elapsed.as_millis()
                ));
                if check.redirects > 0 {
                    result = result.child(div().text_color(rgb(MUTED)).child(format!(
                        "Followed {} {}",
                        check.redirects,
                        if check.redirects == 1 {
                            "redirect"
                        } else {
                            "redirects"
                        }
                    )));
                }
                result
            }
            Status::Failed(error) => div().text_color(rgb(0xf1b6a7)).child(error.clone()),
        };
        let mut buttons = div().flex().gap_2().items_center();
        if self.url.is_some() {
            buttons = buttons.child(Self::button(
                cx,
                "Open in browser".to_owned(),
                true,
                Self::open,
            ));
        } else {
            buttons = buttons.child(Self::button(cx, "Paste link".to_owned(), true, Self::paste));
        }

        let panel = div()
            .w_full()
            .max_w(px(500.0))
            .rounded(px(14.0))
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(0x1f1f22))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .text_sm()
            .child(
                div()
                    .rounded(px(12.0))
                    .bg(rgb(SURFACE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .p_3()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().min_w_0().child(destination))
                    .when(self.url.is_some(), |card| {
                        card.child(Self::button(
                            cx,
                            if self.copied { "Copied" } else { "Copy" }.to_owned(),
                            false,
                            Self::copy,
                        ))
                    }),
            )
            .child(result)
            .when_some(self.error.clone(), |view, error| {
                view.child(div().truncate().text_color(rgb(0xf1b6a7)).child(error))
            })
            .when(self.url.is_some(), |view| {
                let actions = div().flex().gap_2();
                view.child(if matches!(self.status, Status::Working(_)) {
                    actions.h(px(38.0))
                } else {
                    actions
                        .child(Self::button(
                            cx,
                            "Check link".to_owned(),
                            false,
                            Self::check,
                        ))
                        .child(Self::button(
                            cx,
                            "Unshorten".to_owned(),
                            false,
                            Self::unshorten,
                        ))
                        .when(
                            self.url.as_ref().and_then(check::remove_tracking).is_some(),
                            |actions| {
                                actions.child(Self::button(
                                    cx,
                                    "Remove tracking".to_owned(),
                                    false,
                                    Self::remove_tracking,
                                ))
                            },
                        )
                })
            })
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(Self::button(
                                cx,
                                "Settings".to_owned(),
                                false,
                                Self::settings,
                            ))
                            .when(self.original.is_some(), |actions| {
                                actions.child(Self::button(
                                    cx,
                                    "Undo".to_owned(),
                                    false,
                                    Self::undo,
                                ))
                            }),
                    )
                    .child(buttons),
            );

        div()
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &Dismiss, window, _| window.remove_window()))
            .on_action(cx.listener(|view, _: &PasteLink, window, cx| view.paste(window, cx)))
            .on_action(cx.listener(|view, _: &CopyLink, window, cx| view.copy(window, cx)))
            .on_action(cx.listener(|view, _: &CheckLink, window, cx| view.check(window, cx)))
            .on_action(
                cx.listener(|view, _: &UnshortenLink, window, cx| view.unshorten(window, cx)),
            )
            .on_action(
                cx.listener(|view, _: &RemoveTracking, window, cx| {
                    view.remove_tracking(window, cx)
                }),
            )
            .on_action(cx.listener(|view, _: &OpenLink, window, cx| view.open(window, cx)))
            .on_action(cx.listener(|view, _: &UndoLinkChange, window, cx| view.undo(window, cx)))
            .on_action(cx.listener(|view, _: &Settings, window, cx| view.settings(window, cx)))
            .size_full()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(FOREGROUND))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .child(panel)
    }
}

pub fn run(input: Option<String>) {
    gpui_platform::application().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("escape", Dismiss, None),
            KeyBinding::new("ctrl-v", PasteLink, None),
            KeyBinding::new("ctrl-c", CopyLink, None),
            KeyBinding::new("ctrl-enter", CheckLink, None),
            KeyBinding::new("ctrl-u", UnshortenLink, None),
            KeyBinding::new("ctrl-r", RemoveTracking, None),
            KeyBinding::new("ctrl-z", UndoLinkChange, None),
            KeyBinding::new("enter", OpenLink, None),
            KeyBinding::new("ctrl-,", Settings, None),
        ]);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(500.0), px(270.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                app_id: Some("nara".to_owned()),
                titlebar: Some(TitlebarOptions {
                    title: Some("Nara".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(410.0), px(270.0))),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    focus.focus(window, cx);
                    let mut review = Review {
                        url: None,
                        original: None,
                        status: Status::Ready,
                        error: None,
                        copied: false,
                        request: 0,
                        focus,
                    };
                    if let Some(input) = &input {
                        review.set_link(input);
                    }
                    review
                })
            },
        );
        if let Err(error) = result {
            eprintln!("Could not open Nara: {error}");
            cx.quit();
        }
    });
}
