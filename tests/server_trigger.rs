use bevy::{ecs::entity::MapEntities, prelude::*, time::TimePlugin};
use bevy_replicon::{
    prelude::*, shared::server_entity_map::ServerEntityMap, test_app::ServerTestAppExt,
};
use serde::{Deserialize, Serialize};

#[test]
fn sending_receiving() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            RepliconPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .add_server_trigger::<DummyEvent>(Channel::Ordered)
        .finish();
    }
    client_app.init_resource::<TriggerReader<DummyEvent>>();

    server_app.connect_client(&mut client_app);

    server_app.world_mut().server_trigger(ToClients {
        mode: SendMode::Broadcast,
        event: DummyEvent,
    });

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let reader = client_app.world().resource::<TriggerReader<DummyEvent>>();
    assert_eq!(reader.entities, [Entity::PLACEHOLDER]);
}

#[test]
fn sending_receiving_with_target() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            RepliconPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .add_server_trigger::<DummyEvent>(Channel::Ordered)
        .finish();
    }
    client_app.init_resource::<TriggerReader<DummyEvent>>();

    server_app.connect_client(&mut client_app);

    let client_entity = Entity::from_raw(0);
    let server_entity = Entity::from_raw(client_entity.index() + 1);
    client_app
        .world_mut()
        .resource_mut::<ServerEntityMap>()
        .insert(server_entity, client_entity);

    server_app.world_mut().server_trigger_targets(
        ToClients {
            mode: SendMode::Broadcast,
            event: DummyEvent,
        },
        server_entity,
    );

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let reader = client_app.world().resource::<TriggerReader<DummyEvent>>();
    assert_eq!(reader.entities, [client_entity]);
}

#[test]
fn sending_receiving_and_mapping() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    for app in [&mut server_app, &mut client_app] {
        app.add_plugins((
            MinimalPlugins,
            RepliconPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
        ))
        .add_mapped_server_trigger::<EntityEvent>(Channel::Ordered)
        .finish();
    }
    client_app.init_resource::<TriggerReader<EntityEvent>>();

    server_app.connect_client(&mut client_app);

    let client_entity = Entity::from_raw(0);
    let server_entity = Entity::from_raw(client_entity.index() + 1);
    client_app
        .world_mut()
        .resource_mut::<ServerEntityMap>()
        .insert(server_entity, client_entity);

    server_app.world_mut().server_trigger(ToClients {
        mode: SendMode::Broadcast,
        event: EntityEvent(server_entity),
    });

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let reader = client_app.world().resource::<TriggerReader<EntityEvent>>();
    let mapped_entities: Vec<_> = reader.events.iter().map(|event| event.0).collect();
    assert_eq!(mapped_entities, [client_entity]);
}

#[test]
fn sending_receiving_without_plugins() {
    let mut server_app = App::new();
    let mut client_app = App::new();
    server_app
        .add_plugins((
            MinimalPlugins,
            RepliconPlugins
                .build()
                .set(ServerPlugin {
                    tick_policy: TickPolicy::EveryFrame,
                    ..Default::default()
                })
                .disable::<ClientPlugin>()
                .disable::<ClientEventPlugin>(),
        ))
        .add_server_trigger::<DummyEvent>(Channel::Ordered)
        .finish();
    client_app
        .add_plugins((
            MinimalPlugins,
            RepliconPlugins
                .build()
                .disable::<ServerPlugin>()
                .disable::<ServerEventPlugin>(),
        ))
        .add_server_trigger::<DummyEvent>(Channel::Ordered)
        .finish();
    client_app.init_resource::<TriggerReader<DummyEvent>>();

    server_app.connect_client(&mut client_app);

    server_app.world_mut().server_trigger(ToClients {
        mode: SendMode::Broadcast,
        event: DummyEvent,
    });

    server_app.update();
    server_app.exchange_with_client(&mut client_app);
    client_app.update();

    let reader = client_app.world().resource::<TriggerReader<DummyEvent>>();
    assert_eq!(reader.entities.len(), 1);
}

#[test]
fn local_resending() {
    let mut app = App::new();
    app.add_plugins((
        TimePlugin,
        RepliconPlugins.set(ServerPlugin {
            tick_policy: TickPolicy::EveryFrame,
            ..Default::default()
        }),
    ))
    .add_server_trigger::<DummyEvent>(Channel::Ordered)
    .finish();
    app.init_resource::<TriggerReader<DummyEvent>>();

    app.world_mut().server_trigger(ToClients {
        mode: SendMode::Broadcast,
        event: DummyEvent,
    });

    // Requires 2 updates because local resending runs
    // in `PostUpdate` and triggering runs in `PreUpdate`.
    app.update();
    app.update();

    let reader = app.world().resource::<TriggerReader<DummyEvent>>();
    assert_eq!(reader.entities.len(), 1);
}

#[derive(Event, Serialize, Deserialize, Clone)]
struct DummyEvent;

#[derive(Event, Deserialize, Serialize, Clone)]
struct EntityEvent(Entity);

impl MapEntities for EntityEvent {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

#[derive(Resource)]
struct TriggerReader<E: Event> {
    events: Vec<E>,
    entities: Vec<Entity>,
}

impl<E: Event + Clone> FromWorld for TriggerReader<E> {
    fn from_world(world: &mut World) -> Self {
        world.add_observer(|trigger: Trigger<E>, mut counter: ResMut<Self>| {
            counter.events.push(trigger.event().clone());
            counter.entities.push(trigger.entity());
        });

        Self {
            events: Default::default(),
            entities: Default::default(),
        }
    }
}
