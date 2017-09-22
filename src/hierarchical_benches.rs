// Benchmark from https://github.com/lschmierer/ecs_bench

extern crate test;
use self::test::Bencher;
// use std::collections::HashMap;

/// Entities with velocity and position component.
pub const N_POS_VEL: usize = 1000;

/// Entities with position component only.
pub const N_POS: usize = 9000;

// type BenchStorage<T> = ::DenseVec<T>;
// type BenchStorage<T> = ::VecStorage<T>;
type BenchStorage<T> = ::Forest<T>;
// type BenchStorage<T> = HashMap<usize,T>;

// Components
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl<'a> ::Component<'a> for Position{
    type Storage = BenchStorage<Position>;
    type Key = Position;
    fn type_name() -> &'static str{
        "Position"
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

impl<'a> ::Component<'a> for Velocity{
    type Storage = BenchStorage<Velocity>;
    type Key = Velocity;
    fn type_name() -> &'static str{
        "Velocity"
    }
}

// // Systems
// fn physics(entities: ::Entities, _: ::Resources){
//     for (pos, vel) in entities.iter_for::<(::Write<Position>, ::Read<Velocity>)>() {
//         pos.x += vel.dx;
//         pos.y += vel.dy;
//     }
// }
//
// fn render(entities: ::Entities, _: ::Resources){
//     for pos in entities.iter_for::<(::Read<Position>)>() {
//         let _ = pos;
//     }
// }

// Build
fn build<'a>() -> ::World<'a> {
    let mut world = ::World::new();

    world.register_thread_local::<Position>();
    world.register_thread_local::<Velocity>();

    // setup entities
    for _ in 0..N_POS_VEL {
        world.create_entity()
            .add_thread_local(Position { x: 0.0, y: 0.0 })
            .add_thread_local(Velocity { dx: 0.0, dy: 0.0 })
            .build();
    }
    for _ in 0..N_POS {
        world.create_entity()
            .add_thread_local(Position { x: 0.0, y: 0.0 })
            .build();
    }

    // world.add_system(physics);
    // world.add_system(render);
    world
}

// Benchmarks
#[bench]
fn bench_build(b: &mut Bencher) {
    b.iter(build);
}

#[bench]
fn bench_update(b: &mut Bencher) {
    let world = build();

    b.iter(||{
        let entities = world.entities_thread_local();
        // world.run_once();
        for (mut pos, vel) in entities.ordered_iter_for::<(::WriteHierarchical<Position>, ::Read<Velocity>)>(){
            pos.x += vel.dx;
            pos.y += vel.dy;
        }

        for pos in entities.ordered_iter_for::<::ReadHierarchical<Position>>(){
            let _ = pos;
        }
    });
}
