// Benchmark from https://github.com/lschmierer/ecs_bench

extern crate test;
use self::test::Bencher;
// use std::collections::HashMap;

/// Entities with velocity and position component.
pub const N_POS_VEL: usize = 1000;

/// Entities with position component only.
pub const N_POS: usize = 9000;

// Components
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

pub struct Positions{
    p: Vec<Position>
}

impl ::Component for Positions{
    type Storage = ::DenseVec<Positions>;
    fn type_name() -> String{
        "Positions".to_owned()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

pub struct Velocities{
    p: Vec<Velocity>
}

impl ::Component for Velocities{
    type Storage = ::DenseVec<Velocities>;
    fn type_name() -> String{
        "Velocities".to_owned()
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
fn build() -> ::World {
    let mut world = ::World::new();

    world.register_thread_local::<Positions>();
    world.register_thread_local::<Velocities>();

    let mut vec = Vec::new();
    // setup entities
    for _ in 0..N_POS_VEL {
        vec.extend_from_slice(&[Position { x: 0.0, y: 0.0 }, Position { x: 0.0, y: 0.0 }, Position { x: 0.0, y: 0.0 }, Position { x: 0.0, y: 0.0 }]);
        world.create_entity()
            .add_thread_local(Positions { p: vec![Position{x: 0.0, y: 0.0 },Position{x: 0.0, y: 0.0 },Position{x: 0.0, y: 0.0 },Position{x: 0.0, y: 0.0 }] })
            .add_thread_local(Velocities { p: vec![Velocity{ dx: 0.0, dy: 0.0 },Velocity{ dx: 0.0, dy: 0.0 },Velocity{ dx: 0.0, dy: 0.0 },Velocity{ dx: 0.0, dy: 0.0 }] })
            .build();
    }
    for _ in 0..N_POS {
        world.create_entity()
            .add_thread_local(Positions { p: vec![Position{x: 0.0, y: 0.0 }] })
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
        for (poss, vels) in entities.iter_for::<(::Write<Positions>, ::Read<Velocities>)>(){
            for (pos, vel) in poss.p.iter_mut().zip(vels.p.iter()){
                pos.x += vel.dx;
                pos.y += vel.dy;
            }
        }

        for pos in entities.iter_for::<::Read<Positions>>(){
            for pos in &pos.p{
                let _ = pos;
            }
        }
    });
}
