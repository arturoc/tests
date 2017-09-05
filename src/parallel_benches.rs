extern crate test;
extern crate rayon;
use rayon::prelude::*;

use self::test::Bencher;
use std::collections::HashMap;

type BenchStorage<T> = ::DenseVec<T>;
//type BenchStorage<T> = Vec<T>;
//type BenchStorage<T> = HashMap<usize,T>;

pub const N: usize = 10000;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct R {
    pub x: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct W1 {
    pub x: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct W2 {
    pub x: f32,
}

impl ::Component for R{
    type Storage = BenchStorage<R>;
}

impl ::Component for W1{
    type Storage = BenchStorage<W1>;
}

impl ::Component for W2{
    type Storage = BenchStorage<W2>;
}

fn build() -> ::World {
    let mut w = ::World::new();
    w.register::<R>();
    w.register::<W1>();
    w.register::<W2>();

    // setup entities
    {
        for i in 0..N {
            w.create_entity().add(R { x: 0.0 }).build();
            w.create_entity().add(W1 { x: 0.0 }).build();
            w.create_entity().add(W2 { x: 0.0 }).build();
        }
    }

    w
}

fn write_1(w: &::World){
    for (w1, r) in w.iter_for::<(::Write<W1>, ::Read<R>)>() {
        w1.x = r.x;
    }
}

fn write_2(w: &::World){
    for (w2, r) in w.iter_for::<(::Write<W2>, ::Read<R>)>() {
        w2.x = r.x;
    }
}

#[bench]
fn bench_build(b: &mut Bencher) {
    b.iter(|| build());
}

#[bench]
fn bench_update(b: &mut Bencher) {
    let mut world = build();

    b.iter(|| {
        rayon::join(||write_1(&world), ||write_2(&world));
    });
}
