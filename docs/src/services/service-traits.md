# Service Traits: ReadOnlyService and Service

The service abstraction is defined in `src/services/mod.rs`. It provides a standard interface for all backend services.

## UI-agnostic services (`ashell-services`)

Services are being moved to the `ashell-services` workspace crate (`crates/ashell-services/`), which does not depend on iced. There the traits speak plain `futures` types:

```rust
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
```

`src/services/mod.rs` bridges them to iced with blanket impls: every `ashell_services::ReadOnlyService` is also an ashell `ReadOnlyService` (the stream becomes a `Subscription::run_with` keyed by `TypeId`) and every `ashell_services::Service` is also an ashell `Service` (the future becomes a `Task::perform`). Modules keep using the iced traits described below and don't need to know where a service lives. Migrated services are re-exported from `crate::services` (e.g. `pub use ashell_services::bluetooth;`).

Each service sits behind a cargo feature of the same name (`full` enables all of them), so consumers only compile the services, and pull in the system dependencies, they need. The crate has no default features.

Currently migrated: Bluetooth (`bluetooth` feature). Shared helpers that are not services live in plain modules, e.g. `rfkill` (soft-block state and change notifications, used by both bluetooth and network).

## ServiceEvent

All services communicate through a common event enum:

```rust
#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),                    // Service initialized, here's the initial state
    Update(S::UpdateEvent),     // Incremental update
    Error(S::Error),            // Something went wrong
}
```

- **`Init(S)`**: Sent once when the service starts. Contains the full initial state.
- **`Update(S::UpdateEvent)`**: Sent whenever the service state changes. Contains only the change delta.
- **`Error(S::Error)`**: Sent when the service encounters an error.

## ReadOnlyService

For services that only produce events (no commands):

```rust
pub trait ReadOnlyService: Sized {
    type UpdateEvent;
    type Error: Clone;

    fn update(&mut self, event: Self::UpdateEvent);
    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}
```

- **`update()`**: Applies an incremental update to the service state. Called by the module when it receives a `ServiceEvent::Update`.
- **`subscribe()`**: Returns an iced `Subscription` that produces `ServiceEvent<Self>`. This is the event source.

## Service

For services that accept commands (bidirectional):

```rust
pub trait Service: ReadOnlyService {
    type Command: Send + 'static;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>>;
}
```

- **`command()`**: Executes a command and returns a `Task` that may produce a `ServiceEvent`.

Examples of commands:
- `AudioCommand::SetVolume(device, volume)`
- `CompositorCommand::FocusWorkspace(id)`
- `BluetoothCommand::Connect(device_path)`

## Subscription Pattern

Services implement `subscribe()` using iced's `channel` primitive:

```rust
fn subscribe() -> Subscription<ServiceEvent<Self>> {
    Subscription::run_with_id(
        TypeId::of::<Self>(),    // Ensures single instance
        channel(CAPACITY, async move |mut output| {
            // 1. Initialize the service
            let service = MyService::init().await;
            output.send(ServiceEvent::Init(service)).await;

            // 2. Listen for changes in a loop
            loop {
                let event = wait_for_change().await;
                output.send(ServiceEvent::Update(event)).await;
            }
        }),
    )
}
```

Key details:

- **`TypeId::of::<Self>()`**: Each service type gets exactly one subscription instance. If multiple modules subscribe to the same service, they share the same event stream.
- **`channel(capacity, ...)`**: Creates a bounded channel that bridges the async service loop with iced's subscription system.
- The async closure runs for the lifetime of the subscription and continuously sends events.

## Usage in Modules

A module consumes a service like this:

```rust
// In the module's subscription:
fn subscription(&self) -> Subscription<Message> {
    MyService::subscribe().map(|event| Message::ServiceUpdate(event))
}

// In the module's update:
fn update(&mut self, message: Message) {
    match message {
        Message::ServiceUpdate(ServiceEvent::Init(service)) => {
            self.service = Some(service);
        }
        Message::ServiceUpdate(ServiceEvent::Update(event)) => {
            if let Some(service) = &mut self.service {
                service.update(event);
            }
        }
        Message::ServiceUpdate(ServiceEvent::Error(err)) => {
            log::error!("Service error: {err:?}");
        }
    }
}
```
