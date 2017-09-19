
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

#[test]
fn hierarchical_insert_read() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl ::Component for Pos{
        type Storage = ::Forest<Pos>;
    }

    let mut world = ::World::new();
    world.register::<Pos>();
    let e1 = world.create_entity()
        .add(Pos{x: 1., y: 1.})
        .build();
    let e2 = world.create_entity()
        .add(Pos{x: 2., y: 2.})
        .build();
    let e3 = world.create_entity()
        .add_child(e1, Pos{x: 3., y: 3.})
        .build();
    let e4 = world.create_entity()
        .add_child(e2, Pos{x: 4., y: 4.})
        .build();
    let e5 = world.create_entity()
        .add_child(e3, Pos{x: 5., y: 5.})
        .build();

    let entities = world.entities();
    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 5);
    let mut iter = entities.iter_for::<::Read<Pos>>();
    assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
    assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
    assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
    assert_eq!(iter.next(), Some(&Pos{x: 4., y: 4.}));
    assert_eq!(iter.next(), Some(&Pos{x: 5., y: 5.}));
    assert_eq!(iter.next(), None);

    let mut descendants = entities.ordered_iter_for::<::HierarchicalRead<Pos>>();
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 1., y: 1.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 3., y: 3.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 5., y: 5.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 2., y: 2.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 4., y: 4.}));
    assert_eq!(descendants.next().map(|n| n.data), None);
}

#[test]
fn hierarchical_insert_read_write() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    #[derive(Debug,PartialEq,Copy,Clone)]
    struct GlobalPos{
        x: f32,
        y: f32,
    }

    impl ::Component for Pos{
        type Storage = ::Forest<Pos>;
    }

    impl ::Component for GlobalPos{
        type Storage = ::Forest<GlobalPos>;
    }


    let mut world = ::World::new();
    world.register::<Pos>();
    world.register::<GlobalPos>();
    let e1 = world.create_entity()
        .add(Pos{x: 1., y: 1.})
        .add(GlobalPos{x: 1., y: 1.})
        .build();
    let e2 = world.create_entity()
        .add(Pos{x: 2., y: 2.})
        .add(GlobalPos{x: 2., y: 2.})
        .build();
    let e3 = world.create_entity()
        .add_child(e1, Pos{x: 3., y: 3.})
        .add_child(e1, GlobalPos{x: 3., y: 3.})
        .build();
    let e4 = world.create_entity()
        .add_child(e2, Pos{x: 4., y: 4.})
        .add_child(e2, GlobalPos{x: 4., y: 4.})
        .build();
    let e5 = world.create_entity()
        .add_child(e3, Pos{x: 5., y: 5.})
        .add_child(e3, GlobalPos{x: 5., y: 5.})
        .build();

    let entities = world.entities();
    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 5);
    let mut iter = entities.iter_for::<::Read<Pos>>();
    assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
    assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
    assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
    assert_eq!(iter.next(), Some(&Pos{x: 4., y: 4.}));
    assert_eq!(iter.next(), Some(&Pos{x: 5., y: 5.}));
    assert_eq!(iter.next(), None);

    let mut descendants = entities.ordered_iter_for::<::HierarchicalRead<Pos>>();
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 1., y: 1.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 3., y: 3.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 5., y: 5.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 2., y: 2.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 4., y: 4.}));
    assert_eq!(descendants.next().map(|n| n.data), None);

    let mut write_global = entities.ordered_iter_for::<::HierarchicalWrite<GlobalPos>>();
    for mut global_pos in write_global{
        if let Some(parent) = global_pos.parent().map(|p| *p){
            global_pos.x = global_pos.x + parent.x;
            global_pos.y = global_pos.y + parent.y;
        }
    }

    // let mut write_global = entities.ordered_iter_for::<::WriteAndParent<GlobalPos>>();
    // for (mut global_pos, parent) in write_global{
    //     if let Some(parent) = parent{
    //         global_pos.x = global_pos.x + parent.x;
    //         global_pos.y = global_pos.y + parent.y;
    //     }
    // }

    let mut descendants = entities.ordered_iter_for::<::HierarchicalRead<GlobalPos>>();
    assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 1., y: 1.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 4., y: 4.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 9., y: 9.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 2., y: 2.}));
    assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 6., y: 6.}));
    assert_eq!(descendants.next().map(|n| n.data), None);
}
