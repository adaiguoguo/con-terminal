unsafe extern "C" {
    fn con_install_window_cycle_shortcuts();
    fn con_cycle_app_window(reverse: bool);
}

pub fn install_window_cycle_shortcuts() {
    unsafe { con_install_window_cycle_shortcuts() };
}

pub fn cycle_app_window(reverse: bool) {
    unsafe { con_cycle_app_window(reverse) };
}

/// AppKit relocates a window only when its screen disappears. A window left
/// with a sliver on a remaining screen after a display swap stays mostly
/// off-screen, so pull such windows back after every display reconfiguration.
pub fn install_offscreen_window_recovery(cx: &mut gpui::App) {
    recovery::install(cx);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn area(&self) -> f64 {
        self.width.max(0.0) * self.height.max(0.0)
    }

    fn intersection_area(&self, other: &Rect) -> f64 {
        let width = (self.x + self.width).min(other.x + other.width) - self.x.max(other.x);
        let height = (self.y + self.height).min(other.y + other.height) - self.y.max(other.y);
        width.max(0.0) * height.max(0.0)
    }
}

fn recovered_frame(frame: Rect, screens: &[Rect]) -> Option<Rect> {
    let area = frame.area();
    if area <= 0.0 {
        return None;
    }
    // Screens never overlap, so a window spanning several still counts as visible.
    let visible: f64 = screens
        .iter()
        .map(|screen| frame.intersection_area(screen))
        .sum();
    if visible >= area * 0.5 {
        return None;
    }
    // NSScreen.screens lists the menu-bar screen first; prefer it on ties.
    let screen = screens.iter().rev().max_by(|a, b| {
        frame
            .intersection_area(a)
            .total_cmp(&frame.intersection_area(b))
    })?;

    let width = frame.width.min(screen.width);
    let height = frame.height.min(screen.height);
    Some(Rect {
        x: frame.x.clamp(screen.x, screen.x + screen.width - width),
        y: frame.y.clamp(screen.y, screen.y + screen.height - height),
        width,
        height,
    })
}

mod recovery {
    use std::{ffi::c_void, time::Duration};

    use cocoa::foundation::{NSPoint, NSRect, NSSize};
    use gpui::App;
    use objc::{
        class,
        declare::ClassDecl,
        msg_send,
        runtime::{BOOL, Class, NO, Object, Sel, YES},
        sel, sel_impl,
    };

    use super::{Rect, recovered_frame};

    const NS_WINDOW_STYLE_MASK_TITLED: usize = 1 << 0;
    const NS_WINDOW_STYLE_MASK_FULL_SCREEN: usize = 1 << 14;
    // Let AppKit finish its own relocation pass before inspecting frames.
    const SETTLE_DELAY: Duration = Duration::from_millis(300);

    type Sender = futures::channel::mpsc::UnboundedSender<()>;

    extern "C" fn changed(this: &Object, _: Sel, _: *mut Object) {
        unsafe {
            let sender = *this.get_ivar::<*mut c_void>("sender") as *mut Sender;
            let _ = (&*sender).unbounded_send(());
        }
    }

    fn observer_class() -> &'static Class {
        static CLASS: std::sync::OnceLock<&'static Class> = std::sync::OnceLock::new();
        CLASS.get_or_init(|| {
            let mut decl = ClassDecl::new("ConScreenParametersObserver", class!(NSObject)).unwrap();
            decl.add_ivar::<*mut c_void>("sender");
            unsafe {
                decl.add_method(
                    sel!(screenParametersChanged:),
                    changed as extern "C" fn(&Object, Sel, *mut Object),
                );
            }
            decl.register()
        })
    }

    fn to_rect(rect: NSRect) -> Rect {
        Rect {
            x: rect.origin.x,
            y: rect.origin.y,
            width: rect.size.width,
            height: rect.size.height,
        }
    }

    fn recover_windows() {
        unsafe {
            let screens: *mut Object = msg_send![class!(NSScreen), screens];
            let screen_count: usize = msg_send![screens, count];
            let visible_frames: Vec<Rect> = (0..screen_count)
                .map(|index| {
                    let screen: *mut Object = msg_send![screens, objectAtIndex: index];
                    let frame: NSRect = msg_send![screen, visibleFrame];
                    to_rect(frame)
                })
                .collect();

            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let windows: *mut Object = msg_send![app, windows];
            let window_count: usize = msg_send![windows, count];
            for index in 0..window_count {
                let window: *mut Object = msg_send![windows, objectAtIndex: index];
                let visible: BOOL = msg_send![window, isVisible];
                let miniaturized: BOOL = msg_send![window, isMiniaturized];
                let style_mask: usize = msg_send![window, styleMask];
                // Borderless windows (the quick terminal) manage their own frame.
                if visible == NO
                    || miniaturized == YES
                    || style_mask & NS_WINDOW_STYLE_MASK_TITLED == 0
                    || style_mask & NS_WINDOW_STYLE_MASK_FULL_SCREEN != 0
                {
                    continue;
                }

                let frame: NSRect = msg_send![window, frame];
                if let Some(target) = recovered_frame(to_rect(frame), &visible_frames) {
                    let target = NSRect::new(
                        NSPoint::new(target.x, target.y),
                        NSSize::new(target.width, target.height),
                    );
                    let _: () = msg_send![window, setFrame: target display: YES];
                }
            }
        }
    }

    pub(super) fn install(cx: &mut App) {
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<()>();
        cx.spawn(async move |cx| {
            use futures::StreamExt as _;
            while rx.next().await.is_some() {
                cx.background_executor().timer(SETTLE_DELAY).await;
                while rx.try_recv().is_ok() {}
                cx.update(|_| recover_windows());
            }
        })
        .detach();

        unsafe {
            // Lives for the whole app; the notification center keeps no strong reference.
            let observer: *mut Object = msg_send![observer_class(), new];
            (*observer).set_ivar("sender", Box::into_raw(Box::new(tx)) as *mut c_void);
            let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];
            let name: *mut Object = msg_send![class!(NSString), stringWithUTF8String:c"NSApplicationDidChangeScreenParametersNotification".as_ptr()];
            let _: () = msg_send![center, addObserver:observer selector:sel!(screenParametersChanged:) name:name object:std::ptr::null::<Object>()];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Rect, recovered_frame};

    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn keeps_window_that_is_mostly_visible() {
        let screens = [rect(0.0, 0.0, 1512.0, 945.0)];
        assert_eq!(
            recovered_frame(rect(-200.0, 100.0, 1200.0, 800.0), &screens),
            None
        );
    }

    #[test]
    fn keeps_window_spanning_two_screens() {
        let screens = [
            rect(0.0, 0.0, 1512.0, 945.0),
            rect(1512.0, 0.0, 1920.0, 1080.0),
        ];
        assert_eq!(
            recovered_frame(rect(912.0, 100.0, 1200.0, 800.0), &screens),
            None
        );
    }

    #[test]
    fn pulls_sliver_window_back_onto_its_screen() {
        // Only the right 80pt remain on the laptop after the external display changed.
        let screens = [rect(0.0, 0.0, 1512.0, 945.0)];
        let frame = rect(-1120.0, 50.0, 1200.0, 800.0);
        assert_eq!(
            recovered_frame(frame, &screens),
            Some(rect(0.0, 50.0, 1200.0, 800.0))
        );
    }

    #[test]
    fn moves_window_to_screen_with_largest_overlap_and_fits_it() {
        let screens = [
            rect(0.0, 0.0, 1512.0, 945.0),
            rect(1512.0, 0.0, 1280.0, 720.0),
        ];
        let frame = rect(2700.0, 600.0, 1600.0, 900.0);
        assert_eq!(
            recovered_frame(frame, &screens),
            Some(rect(1512.0, 0.0, 1280.0, 720.0))
        );
    }

    #[test]
    fn moves_fully_detached_window_to_the_menu_bar_screen() {
        let screens = [
            rect(0.0, 0.0, 1512.0, 945.0),
            rect(-1920.0, 0.0, 1920.0, 1080.0),
        ];
        let frame = rect(-3000.0, 2000.0, 1200.0, 800.0);
        assert_eq!(
            recovered_frame(frame, &screens),
            Some(rect(0.0, 145.0, 1200.0, 800.0))
        );
    }
}
