mod util;

use bevy::{input::system::exit_on_esc_system, prelude::*, utils::{HashSet, HashMap}};
use itertools::Itertools;
use util::*;

const WIDTH: f32 = 1024.;
const HEIGHT: f32 = 768.;
const TILE_SIZE: f32 = 32.;
const BOARD_DIM: (i32, i32) = (20, 15);
const MINE_NUM: i32 = 40;

const NUM_COLOURS: [Color; 9] = [
    Color::BLACK,
    Color::BLUE,
    Color::GREEN,
    Color::RED,
    Color::MIDNIGHT_BLUE,
    Color::MAROON,
    Color::TEAL,
    Color::BLACK,
    Color::GRAY
];

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    // Menu
    Playing,
    GameOver,
}

// Marker for a tile sprite
#[derive(Component)]
struct Tile;

#[derive(Component)]
struct BoardCoord(Coord);

// Marker for a flag sprite
#[derive(Component)]
struct Flag;

// A 'tile coordinate' representing a tile's position on the board
// This isn't its position in world space, just it's logical position on the board
type Coord = (i32, i32);

// A resource representing the current state of the board
struct BoardState {
    mines: HashSet<Coord>,
    flags: HashSet<Coord>,
    revealed: HashSet<Coord>,
    nums: HashMap<Coord, i32>,
}

// An event triggered when the player clicks on a tile to reveal it
struct RevealEvent;

struct FlagEvent;

fn in_bounds(&(x, y): &Coord) -> bool {
    x >= 0 && y >= 0 && x < BOARD_DIM.0 && y < BOARD_DIM.1
}

// Creates a random board from the constants at the start of the file
fn generate_board() -> BoardState {
    let mut mines = HashSet::new();
    // protect ourselves from adding more mines than there are tiles
    let mut mines_left = MINE_NUM.min(BOARD_DIM.0 * BOARD_DIM.1);

    while mines_left > 0 {
        let coord = (fastrand::i32(0..BOARD_DIM.0), fastrand::i32(0..BOARD_DIM.1));

        if mines.insert(coord) {
            mines_left -= 1;
        }
    }

    let nums = (0..BOARD_DIM.0)
        .cartesian_product(0..BOARD_DIM.1)
        .filter(|p| !mines.contains(p))
        .map(|p| {
            let n = (-1..2)
                .cartesian_product(-1..2)
                .map(|(x, y)| (x + p.0, y + p.1))
                .filter(in_bounds)
                .map(|p| mines.contains(&p) as i32)
                .sum();

            (p, n)
        }).collect();

    BoardState { 
        mines, 
        flags: HashSet::new(), 
        revealed: HashSet::new(),
        nums 
    }
}

fn reveal_board(board: &mut BoardState, pos: Coord) -> bool {
    // You can't reveal a tile that's flagged. Also,
    // if this tile is already revealed dont worry about it and just return
    let mut res = false;

    if !board.flags.contains(&pos) && board.revealed.insert(pos) {
        if board.mines.contains(&pos) {
            // Game over!
            info!("Clicked a mine!!! at position {pos:?}");
            return true;
        } else if let Some(0) = board.nums.get(&pos) {
            // If there are no mines around this tile reveal all the tiles around it
            let to_check = (-1..2)
                .cartesian_product(-1..2)
                .filter(|&p| p != (0,0))
                .map(|(x,y)| (x + pos.0, y + pos.1))
                .filter(in_bounds);

            for neighbour in to_check {
                res |= reveal_board(board, neighbour);
            }
        }
    }

    return res;
}

// Takes screen coordinates and converts them to either Some integer coordinates
// representing which tile the mouse is hovering over, or None if it isn't hovering
// over a tile.
fn pos_to_tile_coords(pos: (f32, f32)) -> Option<Coord> {
    let coords = (
        ((pos.0 - (-0.5 * (BOARD_DIM.0) as f32 * TILE_SIZE)) / TILE_SIZE) as i32,
        ((pos.1 - (-0.5 * (BOARD_DIM.1) as f32 * TILE_SIZE)) / TILE_SIZE) as i32
    );

    if coords.0 >= 0 && coords.1 >= 0 && coords.0 < BOARD_DIM.0 && coords.1 < BOARD_DIM.1 {
        Some(coords)
    } else {
        None
    }
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(CursorPos::default());
    // Load the resources we'll need to create everything
    let font = server.load("fonts/FiraSans-Bold.ttf");
    let tile_spr = server.load("sprites/tile.png");
    let mine_spr = server.load("sprites/mine.png");
    let flag_spr = server.load("sprites/flag.png");
    
    let text_align = TextAlignment {
        horizontal: HorizontalAlign::Center,
        vertical: VerticalAlign::Center,
    };

    let board = generate_board();

    let spawn_tile = |c: &mut ChildBuilder, pos: Coord| {
        c.spawn_bundle(SpriteBundle {
            texture: tile_spr.clone(),
            transform: Transform::from_xyz(pos.0 as f32 * TILE_SIZE, pos.1 as f32 * TILE_SIZE, 1.0),
            ..Default::default()
        })
        .insert(Tile)
        .insert(BoardCoord(pos))
        .with_children(|parent| {
            let num = board.nums.get(&pos).map(|&x| x).unwrap_or_default();
            // The number of mines
            parent.spawn_bundle(Text2dBundle {
                text: Text::with_section(
                    format!("{}", num),
                    TextStyle { font: font.clone(), font_size: 25., color: NUM_COLOURS[num as usize] },
                    text_align
                ),
                visibility: Visibility { is_visible: num != 0 },
                ..default()
            });

            // The mine sprite
            parent.spawn_bundle(SpriteBundle {
                texture: mine_spr.clone(),
                visibility: Visibility { is_visible: board.mines.contains(&pos) },
                transform: Transform {
                    scale: [0.7, 0.7, 1.].into(),
                    ..default()
                },
                ..default()
            });

            // The flag sprite
            // None of the flags are visibile in the beginning
            parent.spawn_bundle(SpriteBundle {
                texture: flag_spr.clone(),
                visibility: Visibility { is_visible: false },
                transform: Transform {
                    translation: [0., 0., 2.].into(),
                    scale: [0.7, 0.7, 1.].into(),
                    ..default()
                },
                ..default()
            })
            .insert(Flag)
            .insert(BoardCoord(pos));
        });
    };

    // Spawn our entities
    // The camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // The board w/ tiles
    commands
        .spawn_bundle(TransformBundle::from_transform(Transform::from_xyz(
            -0.5 * (BOARD_DIM.0 - 1) as f32 * TILE_SIZE,
            -0.5 * (BOARD_DIM.1 - 1) as f32 * TILE_SIZE,
            0.,
        )))
        .with_children(|parent| {
            for y in 0..BOARD_DIM.1 {
                for x in 0..BOARD_DIM.0 {
                    spawn_tile(parent, (x, y));
                }
            }
        });

    // Also insert the board as a resource
    commands.insert_resource(board);
}

fn detect_presses(
    cursor: Res<CursorPos>, 
    mouse_input: Res<Input<MouseButton>>,
    mut board: ResMut<BoardState>,
    mut rew: EventWriter<RevealEvent>,
    mut few: EventWriter<FlagEvent>,
    mut state: ResMut<State<GameState>>,
) {
    if let Some(pos) = pos_to_tile_coords((cursor.x, cursor.y)) {
        if mouse_input.just_pressed(MouseButton::Left) && !board.revealed.contains(&pos) {
            // Reveal the board n stuff
            if reveal_board(&mut board, pos) {
                state.set(GameState::GameOver).unwrap();
            }

            // Also send out an event saying the board has been clicked
            rew.send(RevealEvent);
        } else if mouse_input.just_pressed(MouseButton::Right) && !board.revealed.contains(&pos) {
            // Is there an easier way to just flip whether or not it is a flag or not?
            if !board.flags.insert(pos) {
                board.flags.remove(&pos);
            }

            few.send(FlagEvent);
        }
    }
}

fn update_tile_sprites(
    er: EventReader<RevealEvent>, 
    mut query: Query<(&BoardCoord, &mut Visibility), With<Tile>>,
    board: Res<BoardState>,
) {
    if !er.is_empty() {
        for (coords, mut vis) in query.iter_mut() {
            vis.is_visible = !board.revealed.contains(&coords.0);
        } 
    }
}

fn update_flag_sprites(
    er: EventReader<FlagEvent>,
    mut query: Query<(&BoardCoord, &mut Visibility), With<Flag>>,
    board: Res<BoardState>,
) {
    if !er.is_empty() {
        for (coords, mut vis) in query.iter_mut() {
            vis.is_visible = board.flags.contains(&coords.0);
        }
    }
}

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: "minesweeper!!!".to_string(),
            width: WIDTH,
            height: HEIGHT,
            resizable: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(UtilPlugin)
        .add_state(GameState::Playing)
        .add_event::<RevealEvent>()
        .add_event::<FlagEvent>()
        .add_startup_system(setup)
        .add_system(exit_on_esc_system)
        .add_system_set(SystemSet::on_update(GameState::Playing)
            .with_system(detect_presses)
            .with_system(update_tile_sprites.after(detect_presses))
            .with_system(update_flag_sprites.after(detect_presses))
        )
        .run();
}
