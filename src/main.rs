use bevy::{
    prelude::*, 
    sprite::MaterialMesh2dBundle,
    window::*,
};
use rand::Rng;


// 窗口大小(像素)
const WINDOW_WIDTH: f32 = 500.;
const WINDOW_HEIGHT: f32 = 500.;
// 舞台大小(网格)
const ARENA_WIDTH: i32 = 10;
const ARENA_HEIGHT: i32 = 10;

// 蛇头每次移动的步长(网格)
const MOVE_STEP: i32 = 1;
// 蛇移动时间间隔(秒)
const SNAKE_MOVE_TIK: f32 = 0.2;
// 蛇头颜色
const SNAKE_HEAD_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
// 蛇身体颜色
const SNAKE_SEGMENT_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);

// 食物颜色
const FOOD_COLOR: Color = Color::srgb(1., 0., 1.);
// 食物生成时间间隔(秒)
const FOOD_GEN_TIK: f32 = 2.;


// 方向
#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}
impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

// 蛇头
#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

#[derive(Resource)]
struct SnakeHeadTimer(Timer);

// 蛇身体
#[derive(Component)]
struct SnakeSegment;

#[derive(Default, Resource)]
struct SnakeSegments(Vec<Entity>);

#[derive(Default, Resource)]
struct LastTailPosition(Option<Position>);

// 食物
#[derive(Component)]
struct Food;

#[derive(Resource)]
struct FoodTimer(Timer);

// 位置
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

// 大小（像素）
#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}
impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x, 
        }
    }
}

// 增长事件
#[derive(Event)]
struct GrowthEvent;
// 游戏结束事件
#[derive(Event)]
struct GameOverEvent;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup, spawn_snake).chain())
        .add_systems(Update, ((snake_movement_input, snake_movement, game_over, snake_eating, snake_growth).chain(), food_spawner))
        .add_systems(PostUpdate, (size_scaling, position_translation))
        .insert_resource(FoodTimer(Timer::from_seconds(FOOD_GEN_TIK, TimerMode::Repeating)))
        .insert_resource(SnakeHeadTimer(Timer::from_seconds(SNAKE_MOVE_TIK, TimerMode::Repeating)))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .run();

}


fn setup(
    mut commands: Commands,
) {
    // 生成2d相机
    commands.spawn(Camera2dBundle::default());
}

fn spawn_snake(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut segments: ResMut<SnakeSegments>,
) {
    // 初始化蛇
    *segments = SnakeSegments(vec![
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(Rectangle::default()).into(),
                transform: Transform::default(),
                material: materials.add(SNAKE_HEAD_COLOR),
                ..default()
            },
            SnakeHead {
                direction: Direction::Up,
            },
            SnakeSegment,
            Position {x: 3, y: 3},
            Size::square(0.8)
        )).id(),
        spawn_segment(commands, meshes, materials, Position {x: 3, y: 2}),
    ]);
}


// 蛇身体生成
fn spawn_segment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    position: Position,
) -> Entity {
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Rectangle::default()).into(),
            transform: Transform::default(),
            material: materials.add(SNAKE_SEGMENT_COLOR),
            ..default()
        },
        SnakeSegment,
        position,
        Size::square(0.65)
    )).id()
}


// 缩放
fn size_scaling(windows: Query<&Window, With<PrimaryWindow>>, mut query: Query<(&Size, &mut Transform)>) {
    let window = windows.get_single().unwrap();
    for (size, mut transform) in query.iter_mut() {
        let scale_y = size.height * (window.height() / ARENA_HEIGHT as f32);
        let scale_x = size.width * (window.width() / ARENA_WIDTH as f32);
        transform.scale = Vec3::new(scale_x, scale_y, 1.);
    }
}

// 位置计算
fn position_translation(windows: Query<&Window, With<PrimaryWindow>>, mut query: Query<(&Position, &mut Transform)>) {
    
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let title_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (title_size / 2.)
    }

    let window = windows.get_single().unwrap();
    for (pos, mut transform) in query.iter_mut() {
        let translation_x = convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32);
        let translation_y = convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32);
        transform.translation = Vec3::new(translation_x, translation_y, 0.);
    }
}

// 蛇头方向输入
fn snake_movement_input(
    key_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut SnakeHead>,
) {

    // // 获取蛇头实体的 SnakeHead 组件和 Transform 组件的可变引用。
    // // Transform 就是用来存储蛇头的形态、位置等数据的 Bevy 原生的 Component
    let mut head = query.get_single_mut().unwrap();
    let dir = if key_input.pressed(KeyCode::KeyW) || key_input.pressed(KeyCode::ArrowUp) {
        Direction::Up
    } else if key_input.pressed(KeyCode::KeyS) || key_input.pressed(KeyCode::ArrowDown) {
        Direction::Down
    } else if key_input.pressed(KeyCode::KeyD) || key_input.pressed(KeyCode::ArrowRight) {
        Direction::Right
    } else if key_input.pressed(KeyCode::KeyA) || key_input.pressed(KeyCode::ArrowLeft) {
        Direction::Left
    } else {
        head.direction
    };

    if dir != head.direction.opposite() {
        head.direction =dir;
    }
}


// 蛇头移动函数
fn snake_movement(
    mut head_query: Query<(&SnakeHead, Entity)>, 
    mut position_query: Query<&mut Position>,
    segments: ResMut<SnakeSegments>,
    time: Res<Time>, 
    mut timer: ResMut<SnakeHeadTimer>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut game_over_writer: EventWriter<GameOverEvent>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let (head, head_entity) = head_query.get_single_mut().unwrap();
        let segment_positions = segments.0
            .iter()
            .map(|e| *position_query.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos = position_query.get_mut(head_entity).unwrap();
        // 蛇头方向
        match &head.direction {
            Direction::Up => {
                head_pos.y += MOVE_STEP;
            }
            Direction::Down => {
                head_pos.y -= MOVE_STEP;
            }
            Direction::Right => {
                head_pos.x += MOVE_STEP;
            }
            Direction::Left => {
                head_pos.x -= MOVE_STEP;
            }
        }
        // 是否撞墙游戏结束
        if head_pos.x < 0 || head_pos.y < 0 || head_pos.x >= ARENA_WIDTH as i32 || head_pos.y >= ARENA_HEIGHT as i32 {
            game_over_writer.send(GameOverEvent);
        }
        // 是否撞蛇身结束游戏
        if segment_positions.contains(&head_pos) {
            game_over_writer.send(GameOverEvent);
        }
        // 蛇身跟随
        segment_positions.iter().zip(segments.0.iter().skip(1)).for_each(|(pos, segment)| {
            *position_query.get_mut(*segment).unwrap() = *pos;
        });
        // 记录蛇尾巴
        *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));

        
    }
}

// 食物生成
fn food_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut timer: ResMut<FoodTimer>,
    segments: ResMut<SnakeSegments>,
    mut position_query: Query<&mut Position>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        // 生成不在蛇身上的食物网格坐标
        let segment_positions = segments.0
            .iter()
            .map(|e| *position_query.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let position = loop {
            let new_position = Position {
                x: rand::thread_rng().gen_range(0..ARENA_WIDTH),
                y: rand::thread_rng().gen_range(0..ARENA_HEIGHT),
            };
            if !segment_positions.contains(&new_position) {
                break new_position; // 找到有效位置后跳出循环并返回
            }
        };
        // 生成食物
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(Rectangle::default()).into(),
                transform: Transform::default(),
                material: materials.add(FOOD_COLOR),
                ..default()
            },
            Food,
            position,
            Size::square(0.8)
        ));
    }
}

// 吃食物
fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_position_query: Query<(Entity, &Position), With<Food>>,
    head_position_query: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_position_query.iter() {
        for (ent, food_pos) in food_position_query.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }

}

//  蛇成长
fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    if !growth_reader.is_empty() {
        segments.0.push(spawn_segment(commands, meshes, materials, last_tail_position.0.unwrap()));
        growth_reader.clear();
    }
}

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    if !reader.is_empty() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        reader.clear();
        spawn_snake(commands, meshes, materials, segments_res);
    }
}
