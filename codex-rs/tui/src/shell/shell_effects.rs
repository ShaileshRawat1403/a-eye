use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::tui::FrameRequester;

#[derive(Debug)]
pub(crate) enum UiEffect {
    EmitAppEvent(Box<AppEvent>),
    RequestFrame,
}

pub(crate) fn apply_effects(
    effects: Vec<UiEffect>,
    app_event_tx: &AppEventSender,
    frame_requester: &FrameRequester,
) {
    for effect in effects {
        match effect {
            UiEffect::EmitAppEvent(event) => {
                app_event_tx.send(*event);
            }
            UiEffect::RequestFrame => {
                frame_requester.schedule_frame();
            }
        }
    }
}
