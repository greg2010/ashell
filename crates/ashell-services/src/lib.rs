use futures::Stream;

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
#[cfg(feature = "bluetooth")]
mod stream;

#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),
    Update(S::UpdateEvent),
    Error(S::Error),
}

pub trait ReadOnlyService: Sized + Clone + Send + 'static {
    type UpdateEvent: Send + 'static;
    type Error: Clone + Send + 'static;

    fn update(&mut self, event: Self::UpdateEvent);

    fn subscribe() -> impl Stream<Item = ServiceEvent<Self>> + Send + 'static;
}

pub trait Service: ReadOnlyService {
    type Command: Send + 'static;

    fn command(
        &mut self,
        command: Self::Command,
    ) -> impl Future<Output = ServiceEvent<Self>> + Send + 'static;
}
