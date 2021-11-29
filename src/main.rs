use bevy::prelude::*;
use bevy_spicy_aseprite::{AsepriteAnimation, AsepriteAnimationState, AsepriteBundle, AsepriteImage, AsepritePlugin};

mod sprites {
    use bevy_spicy_aseprite::aseprite;

    aseprite!(pub Player, "assets/char1.ase");
}

#[derive(Component)]
struct PlayerTag;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AsepritePlugin)
        .add_startup_system(setup)
        .add_system(player_input)
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
    commands.spawn_bundle(AsepriteBundle {
        aseprite: sprites::Player::sprite(),
        animation: AsepriteAnimation::from(sprites::Player::tags::LEFT_WALK),
        transform: Transform {
            scale: Vec3::splat(4.),
            translation: Vec3::new(0., -200., 0.),
            ..Default::default()
        },
        ..Default::default()
    }).insert(PlayerTag);
    commands.spawn_bundle(Text2dBundle {
        text: Text {
            alignment: TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
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
                    value: String::from("Ravenfin"),
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
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., 300., 0.)),
        ..Default::default()
    });
}

fn player_input(time: Res<Time>, keys: Res<Input<KeyCode>>, images: Res<Assets<AsepriteImage>>, mut player: Query<(&mut Transform, &mut AsepriteAnimationState, &mut AsepriteAnimation, &Handle<AsepriteImage>), With<PlayerTag>>) {
    let (mut player_trans, mut player_anim_state, mut player_anim, h_img) = player.single_mut();
    
    if keys.pressed(KeyCode::A) {
        //if player_anim.tag
        *player_anim = AsepriteAnimation::from(sprites::Player::tags::LEFT_WALK);
        if player_anim_state.is_paused() {
            player_anim_state.start();
        }
        player_trans.translation.x -= 300.0 * time.delta_seconds();
    }
    else if keys.pressed(KeyCode::D) {
        if !player_anim.is_tag(sprites::Player::tags::RIGHT_WALK) {
            *player_anim = AsepriteAnimation::from(sprites::Player::tags::RIGHT_WALK);
            *player_anim_state = AsepriteAnimationState::default();
        }
        if player_anim_state.is_paused() {
            player_anim_state.start();
        }
        player_trans.translation.x += 300.0 * time.delta_seconds();
    } else {
        if player_anim_state.is_playing() {
            //player_anim_state.pause();
        }
    }
}