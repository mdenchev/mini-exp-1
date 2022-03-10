use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    transform::{transform_propagate_system::transform_propagate_system, TransformSystem},
    utils::HashMap,
};
use bevy_prototype_lyon::{
    entity::ShapeBundle,
    plugin::ShapePlugin,
    prelude::{DrawMode, FillMode, GeometryBuilder, StrokeMode},
    shapes,
};
use bevy_spicy_aseprite::{
    AsepriteAnimation, AsepriteAnimationState, AsepriteBundle, AsepriteImage, AsepritePlugin,
};
use uuid::Uuid;

mod sprites {
    use bevy_spicy_aseprite::aseprite;

    aseprite!(pub Player, "assets/player.ase");
    aseprite!(pub Cow, "assets/cow.ase");
}

const SCALE: f32 = 4.;

#[derive(Component)]
struct PlayerTag;

#[derive(Component)]
struct CowTag;

#[derive(Component)]
struct DebugRenderTag;

#[derive(Component)]
struct ColliderTag;

#[derive(Component)]
struct SensorTag;

#[derive(Component)]
struct Aabb {
    uuid: Uuid,
    extents: Vec2,
}

impl Aabb {
    pub fn extents(&self) -> Vec2 {
        self.extents * SCALE / 2.0 // TODO: why the divide by 2??
    }
}

#[derive(Component, Debug, Clone, Copy)]
enum AabbKind {
    Sensor,
    Collider,
}

#[derive(Debug, Clone, Copy)]
enum CollisionKind {
    SensorSensor,
    ColliderCollider,
    SensorCollider,
}

#[derive(Component, Debug, Clone, Copy)]
enum CollisionBehavior {
    None,
    Static,
    Npc,
    Player,
    Movable,
}

#[derive(Debug, Copy, Clone)]
struct AabbComputed {
    min: Vec2,
    max: Vec2,
    aabb_kind: AabbKind,
    collision_behavior: CollisionBehavior,
}

impl AabbComputed {
    fn intersects(
        &self,
        other: &AabbComputed,
        self_ent: Entity,
        other_ent: Entity,
    ) -> Option<CollisionKind> {
        if self_ent == other_ent {
            return None;
        }

        if ((self.min.x >= other.min.x && self.min.x <= other.max.x)
            || (self.max.x >= other.min.x && self.max.x <= other.max.x))
            && ((self.min.y >= other.min.y && self.min.y <= other.max.y)
                || (self.max.y >= other.min.y && self.max.y <= other.max.y))
        {
            let collision_kind = match (self.aabb_kind, other.aabb_kind) {
                (AabbKind::Collider, AabbKind::Collider) => CollisionKind::ColliderCollider,
                (AabbKind::Collider, AabbKind::Sensor) | (AabbKind::Sensor, AabbKind::Collider) => {
                    CollisionKind::SensorCollider
                }
                (AabbKind::Sensor, AabbKind::Sensor) => CollisionKind::SensorSensor,
            };
            Some(collision_kind)
        } else {
            None
        }
    }

    fn shallow_axis_displace(&self, other: &AabbComputed) -> Vec2 {
        let left_displacement = other.min.x - self.max.x;
        let right_displacement = other.max.x - self.min.x;
        let down_displacement = other.min.y - self.max.y;
        let up_displacement = other.max.y - self.min.y;
        let horizontal = if left_displacement.abs() < right_displacement.abs() {
            left_displacement
        } else {
            right_displacement
        };
        let vertical = if up_displacement.abs() < down_displacement.abs() {
            up_displacement
        } else {
            down_displacement
        };
        if horizontal.abs() < vertical.abs() {
            Vec2::new(horizontal / 2., 0.0)
        } else {
            Vec2::new(0.0, vertical / 2.)
        }
    }
}

#[derive(Bundle)]
struct AabbBundle {
    pub aabb: Aabb,
    pub aabb_kind: AabbKind,
    pub collision_behavior: CollisionBehavior,
    #[bundle]
    pub debug_shape: ShapeBundle,
    pub tag: DebugRenderTag,
}

impl AabbBundle {
    pub fn new(
        extents: Vec2,
        aabb_kind: AabbKind,
        collision_behavior: CollisionBehavior,
        color: Color,
    ) -> Self {
        let shape = shapes::Rectangle {
            extents,
            origin: bevy_prototype_lyon::prelude::RectangleOrigin::Center,
        };

        let builder = GeometryBuilder::new().add(&shape);

        Self {
            aabb: Aabb {
                uuid: Uuid::new_v4(),
                extents,
            },
            aabb_kind,
            collision_behavior,
            debug_shape: builder.build(
                DrawMode::Outlined {
                    fill_mode: FillMode::color(Color::NONE),
                    outline_mode: StrokeMode::color(color),
                },
                Transform::default(),
            ),
            tag: DebugRenderTag,
        }
    }
}

#[derive(Default)]
struct CollisionWorld {
    aabbs: HashMap<Uuid, (Entity, AabbComputed)>,
}

static PHYSICS_STAGE: &str = "physics";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AsepritePlugin)
        .add_plugin(ShapePlugin)
        .add_stage_after(
            CoreStage::PostUpdate,
            PHYSICS_STAGE,
            SystemStage::single_threaded(),
        )
        .init_resource::<CollisionWorld>()
        .add_startup_system(setup)
        .add_system_to_stage(PHYSICS_STAGE, updated_computed_aabbs.label("aabb"))
        .add_system_to_stage(
            PHYSICS_STAGE,
            handle_collision.label("collision").after("aabb"),
        )
        .add_system_to_stage(PHYSICS_STAGE, transform_propagate_system.after("collision"))
        .add_system(bevy::input::system::exit_on_esc_system)
        .add_system(player_input)
        .add_system(toggle_debug_render)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();

    let font = asset_server.load("Share-Regular.ttf");

    let text_style = TextStyle {
        font,
        font_size: 30.,
        ..Default::default()
    };

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(AsepriteBundle {
            aseprite: sprites::Player::sprite(),
            animation: AsepriteAnimation::from(sprites::Player::tags::WEST_WALK),
            transform: Transform {
                scale: Vec3::splat(SCALE),
                translation: Vec3::new(0., -200., 0.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(AabbBundle::new(
                Vec2::new(32., 32.),
                AabbKind::Collider,
                CollisionBehavior::Player,
                Color::GREEN,
            ));
            parent.spawn_bundle(AabbBundle::new(
                Vec2::new(46., 46.),
                AabbKind::Sensor,
                CollisionBehavior::None,
                Color::PURPLE,
            ));
        })
        .insert(PlayerTag);
    commands
        .spawn_bundle(AsepriteBundle {
            aseprite: sprites::Cow::sprite(),
            animation: AsepriteAnimation::from(sprites::Cow::tags::SLEEP),
            transform: Transform {
                scale: Vec3::splat(SCALE),
                translation: Vec3::new(-300., -200., 0.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(AabbBundle::new(
                Vec2::new(32., 32.),
                AabbKind::Collider,
                CollisionBehavior::Static,
                Color::GREEN,
            ));
            parent.spawn_bundle(AabbBundle::new(
                Vec2::new(46., 46.),
                AabbKind::Sensor,
                CollisionBehavior::None,
                Color::PURPLE,
            ));
        })
        .insert(CowTag);
    commands.spawn_bundle(Text2dBundle {
        text: Text {
            alignment: TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Left,
            },
            sections: vec![
                TextSection {
                    value: String::from("Quest: Talk to "),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from("Mrs. Cow"),
                    style: TextStyle {
                        color: Color::LIME_GREEN,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from("."),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style.clone()
                    },
                },
            ],
        },
        transform: Transform::from_translation(Vec3::new(-600., 300., 0.)),
        ..Default::default()
    });
}

fn player_input(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut player: Query<
        (
            &mut Transform,
            &mut AsepriteAnimationState,
            &mut AsepriteAnimation,
            &Handle<AsepriteImage>,
        ),
        With<PlayerTag>,
    >,
) {
    let (mut player_trans, mut player_anim_state, mut player_anim, h_img) = player.single_mut();

    if keys.pressed(KeyCode::A) {
        if !player_anim.is_tag(sprites::Player::tags::WEST_WALK) {
            *player_anim = AsepriteAnimation::from(sprites::Player::tags::WEST_WALK);
        }
        if player_anim_state.is_paused() {
            player_anim_state.start();
        }
        player_trans.translation.x -= 300.0 * time.delta_seconds();
    } else if keys.pressed(KeyCode::D) {
        if !player_anim.is_tag(sprites::Player::tags::EAST_WALK) {
            *player_anim = AsepriteAnimation::from(sprites::Player::tags::EAST_WALK);
        }
        if player_anim_state.is_paused() {
            player_anim_state.start();
        }
        player_trans.translation.x += 300.0 * time.delta_seconds();
    }
    // Trigger idle anim if no input
    else if let AsepriteAnimation::Tag { tag } = *player_anim {
        match tag {
            sprites::Player::tags::EAST_WALK => {
                *player_anim = AsepriteAnimation::from(sprites::Player::tags::EAST_IDLE);
            }
            sprites::Player::tags::WEST_WALK => {
                *player_anim = AsepriteAnimation::from(sprites::Player::tags::WEST_IDLE);
            }
            _ => {}
        }
    }
}

fn toggle_debug_render(
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut Visibility, With<DebugRenderTag>>,
) {
    if keys.just_pressed(KeyCode::Grave) {
        for mut visible in query.iter_mut() {
            visible.is_visible = !visible.is_visible;
        }
    }
}

fn updated_computed_aabbs(
    mut collision_world: ResMut<CollisionWorld>,
    aabb_query: Query<
        (
            &Parent,
            &Aabb,
            &AabbKind,
            &CollisionBehavior,
            &GlobalTransform,
        ),
        Changed<GlobalTransform>,
    >,
) {
    for (parent, aabb, aabb_kind, collision_behavior, g_trans) in aabb_query.iter() {
        let aabb_computed = AabbComputed {
            min: g_trans.translation.xy() - aabb.extents(),
            max: g_trans.translation.xy() + aabb.extents(),
            aabb_kind: *aabb_kind,
            collision_behavior: *collision_behavior,
        };
        collision_world
            .aabbs
            .insert(aabb.uuid, (**parent, aabb_computed));
    }
}

fn handle_collision(
    collision_world: Res<CollisionWorld>,
    mut transform_q: Query<&mut Transform>,
    mut gtransform_q: Query<&mut GlobalTransform>,
) {
    for (ent1, aabb1) in collision_world.aabbs.values() {
        for (ent2, aabb2) in collision_world.aabbs.values() {
            if let Some(collision_kind) = aabb1.intersects(aabb2, *ent1, *ent2) {
                match collision_kind {
                    CollisionKind::ColliderCollider => {
                        match (aabb1.collision_behavior, aabb2.collision_behavior) {
                            (CollisionBehavior::Player, CollisionBehavior::Static) => {
                                let displacement = aabb1.shallow_axis_displace(aabb2);
                                dbg!(&displacement);
                                transform_q
                                    .get_component_mut::<Transform>(*ent1)
                                    .unwrap()
                                    .translation += displacement.extend(0.0);
                                gtransform_q
                                    .get_component_mut::<GlobalTransform>(*ent1)
                                    .unwrap()
                                    .translation += displacement.extend(0.0);
                            }
                            (CollisionBehavior::Static, CollisionBehavior::Player) => {
                                let displacement = aabb2.shallow_axis_displace(aabb1);
                                dbg!(&displacement, ent1, ent2);
                                transform_q
                                    .get_component_mut::<Transform>(*ent2)
                                    .unwrap()
                                    .translation += displacement.extend(0.0);
                                gtransform_q
                                    .get_component_mut::<GlobalTransform>(*ent2)
                                    .unwrap()
                                    .translation += displacement.extend(0.0);
                            }
                            (CollisionBehavior::None, CollisionBehavior::None) => { /* do nothing */
                            }
                            (CollisionBehavior::None, CollisionBehavior::Static) => todo!(),
                            (CollisionBehavior::None, CollisionBehavior::Npc) => todo!(),
                            (CollisionBehavior::None, CollisionBehavior::Player) => todo!(),
                            (CollisionBehavior::None, CollisionBehavior::Movable) => todo!(),
                            (CollisionBehavior::Static, CollisionBehavior::None) => todo!(),
                            (CollisionBehavior::Static, CollisionBehavior::Static) => todo!(),
                            (CollisionBehavior::Static, CollisionBehavior::Npc) => todo!(),
                            (CollisionBehavior::Static, CollisionBehavior::Movable) => todo!(),
                            (CollisionBehavior::Npc, CollisionBehavior::None) => todo!(),
                            (CollisionBehavior::Npc, CollisionBehavior::Static) => todo!(),
                            (CollisionBehavior::Npc, CollisionBehavior::Npc) => todo!(),
                            (CollisionBehavior::Npc, CollisionBehavior::Player) => todo!(),
                            (CollisionBehavior::Npc, CollisionBehavior::Movable) => todo!(),
                            (CollisionBehavior::Player, CollisionBehavior::None) => todo!(),
                            (CollisionBehavior::Player, CollisionBehavior::Npc) => todo!(),
                            (CollisionBehavior::Player, CollisionBehavior::Player) => todo!(),
                            (CollisionBehavior::Player, CollisionBehavior::Movable) => todo!(),
                            (CollisionBehavior::Movable, CollisionBehavior::None) => todo!(),
                            (CollisionBehavior::Movable, CollisionBehavior::Static) => todo!(),
                            (CollisionBehavior::Movable, CollisionBehavior::Npc) => todo!(),
                            (CollisionBehavior::Movable, CollisionBehavior::Player) => todo!(),
                            (CollisionBehavior::Movable, CollisionBehavior::Movable) => todo!(),
                        }
                    }
                    CollisionKind::SensorCollider => {}
                    CollisionKind::SensorSensor => {}
                }
            }
        }
    }
}
