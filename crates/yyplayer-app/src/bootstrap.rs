use std::cell::RefCell;
use std::rc::Rc;

use player_core::{PlaybackCommand, PlaybackEngine};
use player_mpv::MpvEngine;
use player_ui::UiShell;

use crate::controller::AppController;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let platform = player_platform::current();
    let engine = MpvEngine::default();
    let controller = Rc::new(RefCell::new(AppController::new(engine, platform)));
    let ui = UiShell::new()?;
    ui.project(&controller.borrow().view_model());

    let projector = ui.projector();
    let dispatcher = controller.clone();
    ui.bind(Rc::new(move |action| {
        let state = {
            let mut controller = dispatcher.borrow_mut();
            controller.dispatch(action);
            controller.view_model()
        };
        projector(&state);
    }));

    let result = ui.run();
    // No native resources exist yet. Keep an explicit shutdown entry for S03.
    let _ = controller
        .borrow_mut()
        .engine_mut()
        .submit(PlaybackCommand::Shutdown);
    result?;
    Ok(())
}
