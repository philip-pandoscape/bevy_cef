use async_channel::{Receiver, Sender};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_cef_core::prelude::*;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

#[derive(Debug, EntityEvent)]
pub struct Receive<M: Sync + Send + 'static> {
    #[event_target]
    pub webview: Entity,
    pub payload: M,
}

impl<M> Deref for Receive<M>
where
    M: Sync + Send + 'static,
{
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<M> DerefMut for Receive<M>
where
    M: Sync + Send + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.payload
    }
}

pub struct JsEmitEventPlugin<E: DeserializeOwned>(PhantomData<E>);

/// The systems, in `Update`, that take what pages emitted since the last frame and trigger
/// [`Receive`] for each. An app that must hear a page before its own systems run orders this set
/// before them: the observers then run at the sync point between the two.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReceiveJsEmit;

impl<E: DeserializeOwned + Send + Sync + 'static> Plugin for JsEmitEventPlugin<E> {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, receive_events::<E>.in_set(ReceiveJsEmit));
    }
}

impl<E: DeserializeOwned> Default for JsEmitEventPlugin<E> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

fn receive_events<E: DeserializeOwned + Send + Sync + 'static>(
    mut commands: Commands,
    receiver: ResMut<IpcEventRawReceiver>,
) {
    while let Ok(event) = receiver.0.try_recv() {
        if let Ok(payload) = serde_json::from_str::<E>(&event.payload) {
            commands.trigger(Receive {
                webview: event.webview,
                payload,
            });
        }
    }
}

/// What pages have emitted and the app has not taken yet, read without taking it: for an app that
/// hands a page input in the middle of a frame and waits for its answer, working CEF's loop
/// meanwhile, before [`ReceiveJsEmit`] takes it all in that frame as ever. A system that reads it
/// runs on the main thread, where CEF's loop is worked.
#[derive(SystemParam)]
pub struct JsEmitsWaiting<'w> {
    receiver: Res<'w, IpcEventRawReceiver>,
    sender: Res<'w, IpcEventRawSender>,
}

impl JsEmitsWaiting<'_> {
    /// The payloads waiting, oldest first, left where they were: taken and put back in their
    /// order. Nothing adds to the queue meanwhile, as CEF hands a page's messages over only while
    /// its loop is worked, on the thread that reads them here.
    pub fn payloads(&self) -> Vec<String> {
        let mut taken = Vec::new();
        while let Ok(event) = self.receiver.0.try_recv() {
            taken.push(event);
        }
        let payloads = taken.iter().map(|event| event.payload.clone()).collect();
        for event in taken {
            // (Unbounded: never full, and open while the app is.)
            let _ = self.sender.0.try_send(event);
        }
        payloads
    }
}

pub(crate) struct IpcRawEventPlugin;

impl Plugin for IpcRawEventPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = async_channel::unbounded();
        app.insert_resource(IpcEventRawSender(tx))
            .insert_resource(IpcEventRawReceiver(rx));
    }
}

#[derive(Resource)]
pub(crate) struct IpcEventRawSender(pub Sender<IpcEventRaw>);

#[derive(Resource)]
pub(crate) struct IpcEventRawReceiver(pub Receiver<IpcEventRaw>);

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Said {
        n: u32,
    }

    #[derive(Resource, Default)]
    struct Heard(Vec<u32>);

    #[test]
    fn what_pages_emitted_is_read_without_being_taken() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.add_plugins((IpcRawEventPlugin, JsEmitEventPlugin::<Said>::default()))
            .init_resource::<Heard>();
        app.add_observer(|said: On<Receive<Said>>, mut heard: ResMut<Heard>| heard.0.push(said.n));
        let webview = app.world_mut().spawn_empty().id();
        let sender = app.world().resource::<IpcEventRawSender>().0.clone();
        let sent = [r#"{"n":1}"#, "not one", r#"{"n":2}"#];
        for payload in sent {
            sender
                .send_blocking(IpcEventRaw {
                    webview,
                    payload: payload.into(),
                })
                .unwrap();
        }
        let read = |app: &mut App| {
            app.world_mut()
                .run_system_once(|waiting: JsEmitsWaiting| waiting.payloads())
                .unwrap()
        };
        assert_eq!(read(&mut app), sent, "everything waiting, in order");
        assert_eq!(read(&mut app), sent, "read again: still all there");
        app.update();
        assert_eq!(
            app.world().resource::<Heard>().0,
            vec![1, 2],
            "and the frame's own receiving takes it as ever"
        );
        assert!(read(&mut app).is_empty());
    }
}
