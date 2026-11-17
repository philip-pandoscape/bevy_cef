use async_channel::{Receiver, Sender};
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
