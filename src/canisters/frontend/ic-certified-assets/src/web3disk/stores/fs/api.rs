pub struct FsStore;

impl FsStore {}

// pub fn root_node() -> Node {
//     let mut root = Node::Folder(Key::new("/"), HashMap::new());

//     METADATA.with(|tree| {
//         let tree = tree.borrow();

//         for (key, _) in tree.iter() {
//             let mut node = &mut root;

//             for dir_name in key.iter_dir_names() {
//                 match node {
//                     Node::File(_) => panic!("File found in directory tree"),
//                     Node::Folder(_, children) => {
//                         node = children.entry(dir_name).or_insert_with(|| {
//                             Node::Folder(Key::new(&format!("{}/", key.0)), HashMap::new())
//                         });
//                     }
//                 }
//             }

//             match node {
//                 Node::File(_) => panic!("File found in directory tree"),
//                 Node::Folder(_, children) => {
//                     children.insert(key.file_name().unwrap(), Node::File(key.clone()));
//                 }
//             }
//         }
//     });

//     root
// }
