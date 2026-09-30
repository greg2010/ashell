use iced::{
    Subscription, Task,
    futures::{StreamExt, stream},
};
use std::any::TypeId;

pub use ashell_services::bluetooth;

pub mod audio;
pub mod brightness;
pub mod compositor;
pub mod idle_inhibitor;
pub mod logind;
pub mod mpris;
pub mod network;
pub mod notifications;
pub mod privacy;
mod throttle;
pub mod tray;
pub mod upower;
pub mod xdg_icons;

#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),
    Update(S::UpdateEvent),
    Error(S::Error),
}

pub trait Service: ReadOnlyService {
    type Command;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>>;
}

pub trait ReadOnlyService: Sized {
    type UpdateEvent;
    type Error: Clone;

    fn update(&mut self, event: Self::UpdateEvent);

    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}

impl<S: ashell_services::ReadOnlyService> From<ashell_services::ServiceEvent<S>>
    for ServiceEvent<S>
{
    fn from(event: ashell_services::ServiceEvent<S>) -> Self {
        match event {
            ashell_services::ServiceEvent::Init(service) => ServiceEvent::Init(service),
            ashell_services::ServiceEvent::Update(update) => ServiceEvent::Update(update),
            ashell_services::ServiceEvent::Error(error) => ServiceEvent::Error(error),
        }
    }
}

impl<S: ashell_services::ReadOnlyService> ReadOnlyService for S {
    type UpdateEvent = S::UpdateEvent;
    type Error = S::Error;

    fn update(&mut self, event: Self::UpdateEvent) {
        ashell_services::ReadOnlyService::update(self, event);
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<S>(), |_| {
            <S as ashell_services::ReadOnlyService>::subscribe().map(ServiceEvent::from)
        })
    }
}

impl<S: ashell_services::Service> Service for S {
    type Command = S::Command;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        Task::stream(
            stream::once(ashell_services::Service::command(self, command))
                .filter_map(|event| async move { event.map(ServiceEvent::from) }),
        )
    }
}
