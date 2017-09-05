
#[test]
fn insert_read() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl ::Component for Pos{
        type Storage = ::DenseVec<Pos>;
    }

    let mut world = ::World::new();
    world.register::<Pos>();
    world.create_entity()
        .add(Pos{x: 1., y: 1.})
        .build();
    world.create_entity()
        .add(Pos{x: 2., y: 2.})
        .build();
    world.create_entity()
        .add(Pos{x: 3., y: 3.})
        .build();

    let entities = world.entities();
    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);
    let mut iter = entities.iter_for::<::Read<Pos>>();
    assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
    assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
    assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
    assert_eq!(iter.next(), None);
}

#[test]
fn insert_read_write() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl ::Component for Pos{
        type Storage = ::DenseVec<Pos>;
    }

    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl ::Component for Vel{
        type Storage = ::DenseVec<Vel>;
    }

    let mut world = ::World::new();
    world.register::<Pos>();
    world.register::<Vel>();
    world.create_entity()
        .add(Pos{x: 1., y: 1.})
        .add(Vel{x: 1., y: 1.})
        .build();
    world.create_entity()
        .add(Pos{x: 2., y: 2.})
        .add(Vel{x: 1., y: 1.})
        .build();
    world.create_entity()
        .add(Pos{x: 3., y: 3.})
        .add(Vel{x: 1., y: 1.})
        .build();

    let entities = world.entities();
    for (pos, vel) in entities.iter_for::<(::Write<Pos>, ::Read<Vel>)>(){
        pos.x += vel.x;
        pos.y += vel.y;
    }

    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);
    let mut iter = entities.iter_for::<::Read<Pos>>();
    assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
    assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
    assert_eq!(iter.next(), Some(&Pos{x: 4., y: 4.}));
    assert_eq!(iter.next(), None);
}

#[test]
fn insert_read_write_parallel() {
    use rayon;

    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl ::Component for Pos{
        type Storage = ::DenseVec<Pos>;
    }

    struct C1;
    impl ::Component for C1{
        type Storage = ::DenseVec<C1>;
    }

    struct C2;
    impl ::Component for C2{
        type Storage = ::DenseVec<C2>;
    }

    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl ::Component for Vel{
        type Storage = ::DenseVec<Vel>;
    }

    let mut world = ::World::new();

    world.register::<Pos>();
    world.register::<Vel>();
    world.register::<C1>();
    world.register::<C2>();

    for i in 0..100usize{
        world.create_entity()
            .add(Pos{x: i as f32, y: i as f32})
            .add(C1)
            .add(Vel{x: 1., y: 1.})
            .build();
    }

    for i in 0..100usize{
        world.create_entity()
            .add(Pos{x: i as f32, y: i as f32})
            .add(C2)
            .add(Vel{x: 1., y: 1.})
            .build();
    }

    fn write1(w: ::Entities){
        for (pos, _, vel) in w.iter_for::<(::Write<Pos>, ::Read<C1>, ::Read<Vel>)>(){
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }

    fn write2(w: ::Entities){
        for (pos, _, vel) in w.iter_for::<(::Write<Pos>, ::Read<C2>, ::Read<Vel>)>(){
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }

    let entities1 = world.entities();
    let entities2 = world.entities();
    rayon::join(||write1(entities1), ||write2(entities2));

    let entities = world.entities_thread_local();
    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 200);
    let mut iter = entities.iter_for::<::Read<Pos>>();
    for i in 0..100{
        assert_eq!(iter.next(), Some(&Pos{x: (i + 1) as f32, y: (i + 1) as f32}));
    }
    for i in 0..100{
        assert_eq!(iter.next(), Some(&Pos{x: (i + 1) as f32, y: (i + 1) as f32}));
    }
}