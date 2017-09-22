
#[test]
fn insert_read() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
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
fn insert_read_entities() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
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
    assert_eq!(entities.iter_for::<(::ReadEntities,::Read<Pos>)>().count(), 3);
    let mut iter = entities.iter_for::<(::ReadEntities,::Read<Pos>)>();
    let next = iter.next().unwrap();
    assert_eq!(next.0.guid(), 0);
    assert_eq!(next.1, &Pos{x: 1., y: 1.});

    let next = iter.next().unwrap();
    assert_eq!(next.0.guid(), 1);
    assert_eq!(next.1, &Pos{x: 2., y: 2.});

    let next = iter.next().unwrap();
    assert_eq!(next.0.guid(), 2);
    assert_eq!(next.1, &Pos{x: 3., y: 3.});

    assert_eq!(iter.next(), None);
}

#[test]
fn insert_read_write() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Vel{
        type Storage = ::DenseVec<Vel>;
        type Key = Vel;
        fn type_name() -> &'static str{
            "Vel"
        }
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

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    struct C1;
    impl<'a> ::Component<'a> for C1{
        type Storage = ::DenseVec<C1>;
        type Key = C1;
        fn type_name() -> &'static str{
            "C1"
        }
    }

    struct C2;
    impl<'a> ::Component<'a> for C2{
        type Storage = ::DenseVec<C2>;
        type Key = C2;
        fn type_name() -> &'static str{
            "C2"
        }
    }

    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Vel{
        type Storage = ::DenseVec<Vel>;
        type Key = Vel;
        fn type_name() -> &'static str{
            "Vel"
        }
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

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::Forest<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
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
    let _e4 = world.create_entity()
        .add_child(e2, Pos{x: 4., y: 4.})
        .build();
    let _e5 = world.create_entity()
        .add_child(e3, Pos{x: 5., y: 5.})
        .build();

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 5);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), Some(&Pos{x: 4., y: 4.}));
        assert_eq!(iter.next(), Some(&Pos{x: 5., y: 5.}));
        assert_eq!(iter.next(), None);

        let mut descendants = entities.ordered_iter_for::<::ReadHierarchical<Pos>>();
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 1., y: 1.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 3., y: 3.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 5., y: 5.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 2., y: 2.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 4., y: 4.}));
        assert_eq!(descendants.next().map(|n| n.data), None);
    }
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

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::Forest<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::Component<'a> for GlobalPos{
        type Storage = ::Forest<GlobalPos>;
        type Key = GlobalPos;
        fn type_name() -> &'static str{
            "GlobalPos"
        }
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
    let _e4 = world.create_entity()
        .add_child(e2, Pos{x: 4., y: 4.})
        .add_child(e2, GlobalPos{x: 4., y: 4.})
        .build();
    let _e5 = world.create_entity()
        .add_child(e3, Pos{x: 5., y: 5.})
        .add_child(e3, GlobalPos{x: 5., y: 5.})
        .build();
    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 5);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), Some(&Pos{x: 4., y: 4.}));
        assert_eq!(iter.next(), Some(&Pos{x: 5., y: 5.}));
        assert_eq!(iter.next(), None);

        let mut descendants = entities.ordered_iter_for::<::ReadHierarchical<Pos>>();
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 1., y: 1.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 3., y: 3.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 5., y: 5.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 2., y: 2.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 4., y: 4.}));
        assert_eq!(descendants.next().map(|n| n.data), None);

        let write_global = entities.ordered_iter_for::<::WriteHierarchical<GlobalPos>>();
        for mut global_pos in write_global{
            if let Some(parent) = global_pos.parent().map(|p| *p){
                global_pos.x = global_pos.x + parent.x;
                global_pos.y = global_pos.y + parent.y;
            }
        }

        // let write_global = entities.ordered_iter_for::<::WriteAndParent<GlobalPos>>();
        // for (mut global_pos, parent) in write_global{
        //     if let Some(parent) = parent{
        //         global_pos.x = global_pos.x + parent.x;
        //         global_pos.y = global_pos.y + parent.y;
        //     }
        // }

        let mut descendants = entities.ordered_iter_for::<::ReadHierarchical<GlobalPos>>();
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 1., y: 1.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 4., y: 4.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 9., y: 9.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 2., y: 2.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 6., y: 6.}));
        assert_eq!(descendants.next().map(|n| n.data), None);
    }
}



#[test]
fn read_write_and_parent() {
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

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::Forest<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::Component<'a> for GlobalPos{
        type Storage = ::Forest<GlobalPos>;
        type Key = GlobalPos;
        fn type_name() -> &'static str{
            "GlobalPos"
        }
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
    let _e4 = world.create_entity()
        .add_child(e2, Pos{x: 4., y: 4.})
        .add_child(e2, GlobalPos{x: 4., y: 4.})
        .build();
    let _e5 = world.create_entity()
        .add_child(e3, Pos{x: 5., y: 5.})
        .add_child(e3, GlobalPos{x: 5., y: 5.})
        .build();

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 5);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), Some(&Pos{x: 4., y: 4.}));
        assert_eq!(iter.next(), Some(&Pos{x: 5., y: 5.}));
        assert_eq!(iter.next(), None);

        let mut descendants = entities.ordered_iter_for::<::ReadHierarchical<Pos>>();
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 1., y: 1.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 3., y: 3.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 5., y: 5.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 2., y: 2.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(Pos{x: 4., y: 4.}));
        assert_eq!(descendants.next().map(|n| n.data), None);

        let write_global = entities.ordered_iter_for::<::WriteAndParent<GlobalPos>>();
        for (global_pos, parent) in write_global{
            if let Some(parent) = parent{
                global_pos.x = global_pos.x + parent.x;
                global_pos.y = global_pos.y + parent.y;
            }
        }

        let mut descendants = entities.ordered_iter_for::<::ReadHierarchical<GlobalPos>>();
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 1., y: 1.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 4., y: 4.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 9., y: 9.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 2., y: 2.}));
        assert_eq!(descendants.next().map(|n| n.data), Some(GlobalPos{x: 6., y: 6.}));
        assert_eq!(descendants.next().map(|n| n.data), None);
    }
}

#[test]
fn insert_remove_dense_vec() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::Component<'a> for Vel{
        type Storage = ::DenseVec<Vel>;
        type Key = Vel;
        fn type_name() -> &'static str{
            "Vel"
        }
    }

    let mut world = ::World::new();
    world.register::<Pos>();
    world.register::<Vel>();
    let _e1 = world.create_entity()
        .add(Pos{x: 1., y: 1.})
        .add(Vel{x: 1., y: 1.})
        .build();
    let e2 = world.create_entity()
        .add(Pos{x: 2., y: 2.})
        .add(Vel{x: 2., y: 2.})
        .build();
    let _e3 = world.create_entity()
        .add(Pos{x: 3., y: 3.})
        .add(Vel{x: 3., y: 3.})
        .build();

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 3);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 2., y: 2.}, &Vel{x: 2., y: 2.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);
    }

    world.remove_component_from::<Vel>(&e2);

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 2);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);

        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

    world.remove_entity(&e2);

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 2);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);

        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 2);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

}



#[test]
fn insert_remove_vec() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::VecStorage<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::Component<'a> for Vel{
        type Storage = ::VecStorage<Vel>;
        type Key = Vel;
        fn type_name() -> &'static str{
            "Vel"
        }
    }

    let mut world = ::World::new();
    world.register::<Pos>();
    world.register::<Vel>();
    let _e1 = world.create_entity()
        .add(Pos{x: 1., y: 1.})
        .add(Vel{x: 1., y: 1.})
        .build();
    let e2 = world.create_entity()
        .add(Pos{x: 2., y: 2.})
        .add(Vel{x: 2., y: 2.})
        .build();
    let _e3 = world.create_entity()
        .add(Pos{x: 3., y: 3.})
        .add(Vel{x: 3., y: 3.})
        .build();

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 3);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 2., y: 2.}, &Vel{x: 2., y: 2.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);
    }

    world.remove_component_from::<Vel>(&e2);

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 2);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);

        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

    world.remove_entity(&e2);

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 2);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);

        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 2);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

}



#[test]
fn insert_remove_forest() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::Forest<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::Component<'a> for Vel{
        type Storage = ::Forest<Vel>;
        type Key = Vel;
        fn type_name() -> &'static str{
            "Vel"
        }
    }

    let mut world = ::World::new();
    world.register::<Pos>();
    world.register::<Vel>();
    let _e1 = world.create_entity()
        .add(Pos{x: 1., y: 1.})
        .add(Vel{x: 1., y: 1.})
        .build();
    let e2 = world.create_entity()
        .add(Pos{x: 2., y: 2.})
        .add(Vel{x: 2., y: 2.})
        .build();
    let _e3 = world.create_entity()
        .add(Pos{x: 3., y: 3.})
        .add(Vel{x: 3., y: 3.})
        .build();

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 3);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 2., y: 2.}, &Vel{x: 2., y: 2.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);
    }

    world.remove_component_from::<Vel>(&e2);

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 2);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);

        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

    world.remove_entity(&e2);

    {
        let entities = world.entities();
        assert_eq!(entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>().count(), 2);
        let mut iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
        assert_eq!(iter.next(), Some((&Pos{x: 1., y: 1.}, &Vel{x: 1., y: 1.})));
        assert_eq!(iter.next(), Some((&Pos{x: 3., y: 3.}, &Vel{x: 3., y: 3.})));
        assert_eq!(iter.next(), None);

        assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 2);
        let mut iter = entities.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

}

#[test]
fn insert_read_one_to_n() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseOneToNVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::OneToNComponent<'a> for Pos{}

    let mut world = ::World::new();
    world.register::<Pos>();
    world.create_entity()
        .add_slice(&[Pos{x: 1., y: 1.}])
        .build();
    world.create_entity()
        .add_slice(&[Pos{x: 2., y: 2.}, Pos{x: 2., y: 2.}])
        .build();
    world.create_entity()
        .add_slice(&[Pos{x: 3., y: 3.}, Pos{x: 3., y: 3.}, Pos{x: 3., y: 3.}])
        .build();

    let entities = world.entities();
    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);

    let iter = entities.iter_for::<::Read<Pos>>();
    for poss in iter{
        assert_eq!(poss[0], Pos{x: poss.len() as f32, y: poss.len() as f32});
    }
}


#[test]
fn insert_read_write_one_to_n() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseOneToNVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::OneToNComponent<'a> for Pos{}

    let mut world = ::World::new();
    world.register::<Pos>();
    world.create_entity()
        .add_slice(&[Pos{x: 1., y: 1.}])
        .build();
    world.create_entity()
        .add_slice(&[Pos{x: 2., y: 2.}, Pos{x: 2., y: 2.}])
        .build();
    world.create_entity()
        .add_slice(&[Pos{x: 3., y: 3.}, Pos{x: 3., y: 3.}, Pos{x: 3., y: 3.}])
        .build();

    let entities = world.entities();
    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);


    let iter = entities.iter_for::<::Write<Pos>>();
    for poss in iter{
        for pos in poss{
            pos.x += 1.;
            pos.y += 1.;
        }
    }


    let iter = entities.iter_for::<::Read<Pos>>();
    for poss in iter{
        assert_eq!(poss[0], Pos{x: poss.len() as f32 + 1., y: poss.len() as f32 + 1.});
    }
}


#[test]
fn insert_read_combined_one_to_n() {
    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Pos{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Pos{
        type Storage = ::DenseOneToNVec<Pos>;
        type Key = Pos;
        fn type_name() -> &'static str{
            "Pos"
        }
    }

    impl<'a> ::OneToNComponent<'a> for Pos{}

    #[derive(Debug,PartialEq,Copy,Clone)]
    struct Vel{
        x: f32,
        y: f32,
    }

    impl<'a> ::Component<'a> for Vel{
        type Storage = ::DenseVec<Vel>;
        type Key = Vel;
        fn type_name() -> &'static str{
            "Vel"
        }
    }

    let mut world = ::World::new();
    world.register::<Pos>();
    world.register::<Vel>();
    world.create_entity()
        .add_slice(&[Pos{x: 1., y: 1.}])
        .add(Vel{x: 1., y: 1.})
        .build();
    world.create_entity()
        .add_slice(&[Pos{x: 2., y: 2.}, Pos{x: 2., y: 2.}])
        .add(Vel{x: 1., y: 1.})
        .build();
    world.create_entity()
        .add_slice(&[Pos{x: 3., y: 3.}, Pos{x: 3., y: 3.}, Pos{x: 3., y: 3.}])
        .add(Vel{x: 1., y: 1.})
        .build();

    let entities = world.entities();
    assert_eq!(entities.iter_for::<::Read<Pos>>().count(), 3);

    let iter = entities.iter_for::<(::Read<Pos>, ::Read<Vel>)>();
    for (poss, vel) in iter{
        assert_eq!(poss[0], Pos{x: poss.len() as f32, y: poss.len() as f32});
        assert_eq!(vel, &Vel{x: 1., y: 1.});
    }
}

#[test]
fn insert_read_slice_alloc() {
    #[derive(Copy,Clone,PartialEq,Debug)]
    struct Vertex{
        x: f32, y: f32,
    }

    #[derive(Copy,Clone)]
    struct Vertices<'a>(&'a [Vertex]);

    impl<'a> ::Component<'a> for Vertices<'a>{
        type Storage = ::DenseVec<Vertices<'a>>;
        type Key = Vertices<'static>;
        fn type_name() -> &'static str{
            "Vertices"
        }
    }

    let mut alloc = Vec::new();

    let mut world = ::World::new();
    world.register::<Vertices>();

    alloc.extend_from_slice(&[Vertex{x: 1., y: 1.}]);
    alloc.extend_from_slice(&[Vertex{x: 2., y: 2.}, Vertex{x: 2., y: 2.}]);
    alloc.extend_from_slice(&[Vertex{x: 3., y: 3.}, Vertex{x: 3., y: 3.}, Vertex{x: 3., y: 3.}]);

    world.create_entity()
        .add(Vertices(&alloc[0..1]))
        .build();

    world.create_entity()
        .add(Vertices(&alloc[1..3]))
        .build();

    world.create_entity()
        .add(Vertices(&alloc[3..6]))
        .build();

    let entities = world.entities();
    assert_eq!(entities.iter_for::<::Read<Vertices>>().count(), 3);

    let iter = entities.iter_for::<::Read<Vertices>>();
    for poss in iter{
        assert_eq!(poss.0[0], Vertex{x: poss.0.len() as f32, y: poss.0.len() as f32});
    }
}
