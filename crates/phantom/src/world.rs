// ECS

#[derive(Default)]
pub struct World {
    pub entity_allocator: EntityAllocator,
    pub resources: AnyMap,
    pub components: AnyMap, // Holds `EntityMap<T>`
    pub scenes: Vec<Scene>,
}

pub struct Scene {
    pub name: String,
    pub graph: SceneGraph,
}

pub type SceneGraph = petgraph::Graph<Entity, ()>;

pub type Entity = GenerationalIndex;
pub type EntityMap<T> = GenerationalVec<T>;
pub type EntityAllocator = HandleAllocator;

#[derive(
    Default, Debug, PartialEq, Eq, Copy, Clone, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct GenerationalIndex {
    pub index: usize,
    pub generation: u64,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Slot<T> {
    pub value: T,
    pub generation: u64,
}

pub type SlotVec<T> = Vec<Option<Slot<T>>>;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct GenerationalVec<T> {
    elements: SlotVec<T>,
}

impl<T> GenerationalVec<T> {
    pub fn new(elements: SlotVec<T>) -> Self {
        Self { elements }
    }

    pub fn insert(&mut self, handle: GenerationalIndex, value: T) {
        while self.elements.len() <= handle.index {
            self.elements.push(None);
        }

        let previous_generation = match self.elements.get(handle.index) {
            Some(Some(entry)) => entry.generation,
            _ => 0,
        };

        if previous_generation > handle.generation {
            return;
        }

        self.elements[handle.index] = Some(Slot {
            value,
            generation: handle.generation,
        });
    }

    pub fn remove(&mut self, handle: GenerationalIndex) {
        if let Some(e) = self.elements.get_mut(handle.index) {
            *e = None;
        }
    }

    pub fn get(&self, handle: GenerationalIndex) -> Option<&T> {
        if handle.index >= self.elements.len() {
            return None;
        }
        self.elements[handle.index]
            .as_ref()
            .filter(|c| c.generation == handle.generation)
            .map(|entry| &entry.value)
    }

    pub fn get_mut(&mut self, handle: GenerationalIndex) -> Option<&mut T> {
        if handle.index >= self.elements.len() {
            return None;
        }
        self.elements[handle.index]
            .as_mut()
            .filter(|c| c.generation == handle.generation)
            .map(|entry| &mut entry.value)
    }
}

#[derive(Default, Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Allocation {
    pub allocated: bool,
    pub generation: u64,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HandleAllocator {
    allocations: Vec<Allocation>,
    available_handles: Vec<usize>,
}

impl HandleAllocator {
    pub fn allocate(&mut self) -> GenerationalIndex {
        match self.available_handles.pop() {
            Some(index) => {
                self.allocations[index].generation += 1;
                self.allocations[index].allocated = true;
                GenerationalIndex {
                    index,
                    generation: self.allocations[index].generation,
                }
            }
            None => {
                self.allocations.push(Allocation {
                    allocated: true,
                    generation: 0,
                });
                GenerationalIndex {
                    index: self.allocations.len() - 1,
                    generation: 0,
                }
            }
        }
    }

    pub fn deallocate(&mut self, handle: &GenerationalIndex) {
        if !self.is_allocated(handle) {
            return;
        }
        self.allocations[handle.index].allocated = false;
        self.available_handles.push(handle.index);
    }

    pub fn is_allocated(&self, handle: &GenerationalIndex) -> bool {
        self.handle_exists(handle)
            && self.allocations[handle.index].generation == handle.generation
            && self.allocations[handle.index].allocated
    }

    pub fn handle_exists(&self, handle: &GenerationalIndex) -> bool {
        handle.index < self.allocations.len()
    }

    pub fn allocated_handles(&self) -> Vec<GenerationalIndex> {
        self.allocations
            .iter()
            .enumerate()
            .filter(|(_, allocation)| allocation.allocated)
            .map(|(index, allocation)| GenerationalIndex {
                index,
                generation: allocation.generation,
            })
            .collect()
    }
}

#[derive(Default)]
pub struct AnyMap {
    data: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + 'static>>,
}

impl AnyMap {
    /// Retrieve the value stored in the map for the type `T`, if it exists.
    pub fn find<T: 'static>(&self) -> Option<&T> {
        self.data
            .get(&std::any::TypeId::of::<T>())
            .and_then(|any| any.downcast_ref())
    }

    /// Retrieve a mutable reference to the value stored in the map for the type `T`, if it exists.
    pub fn find_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&std::any::TypeId::of::<T>())
            .and_then(|any| any.downcast_mut())
    }

    /// Set the value contained in the map for the type `T`.
    /// This will override any previous value stored.
    pub fn insert<T: 'static>(&mut self, value: T) {
        self.data.insert(
            std::any::TypeId::of::<T>(),
            Box::new(value) as Box<dyn std::any::Any + 'static>,
        );
    }

    /// Remove the value for the type `T` if it existed.
    pub fn remove<T: 'static>(&mut self) {
        self.data.remove(&std::any::TypeId::of::<T>());
    }
}

pub struct AnyMapIter<'a> {
    iter: std::collections::hash_map::Iter<'a, std::any::TypeId, Box<dyn std::any::Any + 'static>>,
}

impl<'a> IntoIterator for &'a AnyMap {
    type Item = (&'a std::any::TypeId, &'a Box<dyn std::any::Any + 'static>);
    type IntoIter = AnyMapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AnyMapIter {
            iter: self.data.iter(),
        }
    }
}

impl<'a> Iterator for AnyMapIter<'a> {
    type Item = (&'a std::any::TypeId, &'a Box<dyn std::any::Any + 'static>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_and_removal() {
        let mut elements = GenerationalVec::new(SlotVec::<u32>::default());
        let mut handle_allocator = HandleAllocator::default();

        // allocate a handle
        let handle = handle_allocator.allocate();
        elements.insert(handle, 3);
        assert_eq!(elements.get(handle), Some(&3));

        // modify an existing handle
        if let Some(element) = elements.get_mut(handle) {
            *element = 10;
        }
        assert_eq!(elements.get(handle), Some(&10));

        // Clear a handle's slot
        elements.remove(handle);
        assert_eq!(elements.get(handle), None);

        // Deallocate a handle
        handle_allocator.deallocate(&handle);
        assert!(!handle_allocator.is_allocated(&handle));

        // This assures that the "A->B->A" problem is addressed
        let next_handle = handle_allocator.allocate();
        assert_eq!(
            next_handle,
            GenerationalIndex {
                index: handle.index,
                generation: handle.generation + 1,
            }
        );
    }

    #[test]
    fn allocated_handles() {
        let mut handle_allocator = HandleAllocator::default();

        let first_handle = handle_allocator.allocate();
        assert!(handle_allocator.is_allocated(&first_handle));
        assert_eq!(handle_allocator.allocated_handles(), &[first_handle]);

        let second_handle = handle_allocator.allocate();
        assert!(handle_allocator.is_allocated(&second_handle));
        assert_eq!(
            handle_allocator.allocated_handles(),
            &[first_handle, second_handle]
        );
    }

    #[test]
    fn test_insert() {
        let mut vec = GenerationalVec::new(Vec::new());

        let handle = HandleAllocator::default().allocate();
        vec.insert(handle, 10);

        assert_eq!(*vec.get(handle).unwrap(), 10);
    }

    #[test]
    fn test_remove() {
        let mut vec = GenerationalVec::new(Vec::new());

        let handle = HandleAllocator::default().allocate();
        vec.insert(handle, 10);

        vec.remove(handle);

        assert!(vec.get(handle).is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut vec = GenerationalVec::new(Vec::new());

        let handle = HandleAllocator::default().allocate();
        vec.insert(handle, 10);

        *vec.get_mut(handle).unwrap() = 20;

        assert_eq!(*vec.get(handle).unwrap(), 20);
    }

    #[test]
    fn test_invalid_generation() {
        let mut vec = GenerationalVec::new(Vec::new());

        let handle = HandleAllocator::default().allocate();
        vec.insert(handle, 10);

        // Modify the handle to have an invalid generation
        let invalid_handle = GenerationalIndex {
            generation: handle.generation + 1,
            ..handle
        };

        assert!(vec.get(invalid_handle).is_none());
        assert!(vec.get_mut(invalid_handle).is_none());
    }

    #[test]
    fn test_generational_vec() -> Result<(), Box<dyn std::error::Error>> {
        let mut allocator = HandleAllocator::default();
        let handle1 = allocator.allocate();
        let handle2 = allocator.allocate();
        let handle3 = allocator.allocate();

        let mut vec = GenerationalVec::new(Vec::new());

        assert!(vec.get(handle1).is_none());
        assert!(vec.get(handle2).is_none());
        assert!(vec.get(handle3).is_none());

        vec.insert(handle1, "value1".to_string());
        vec.insert(handle2, "value2".to_string());
        vec.insert(handle3, "value3".to_string());

        assert_eq!(vec.get(handle1), Some(&"value1".to_string()));
        assert_eq!(vec.get(handle2), Some(&"value2".to_string()));
        assert_eq!(vec.get(handle3), Some(&"value3".to_string()));

        vec.remove(handle1);
        assert!(vec.get(handle1).is_none());
        assert_eq!(vec.get(handle2), Some(&"value2".to_string()));
        assert_eq!(vec.get(handle3), Some(&"value3".to_string()));

        allocator.deallocate(&handle1);
        allocator.deallocate(&handle2);
        allocator.deallocate(&handle3);

        assert!(!allocator.is_allocated(&handle1));
        assert!(!allocator.is_allocated(&handle2));
        assert!(!allocator.is_allocated(&handle3));

        Ok(())
    }

    struct EntryA {
        pub value: u32,
    }

    struct EntryB {
        pub message: String,
    }

    #[test]
    fn anymap() {
        let mut anymap = AnyMap::default();
        anymap.insert(EntryA { value: 3 });
        assert_eq!(anymap.find::<EntryA>().unwrap().value, 3);

        if let Some(entry) = anymap.find_mut::<EntryA>() {
            entry.value = 10;
        }
        assert_eq!(anymap.find::<EntryA>().unwrap().value, 10);

        anymap.insert(EntryB {
            message: "Hi!".to_string(),
        });
        assert_eq!(anymap.find::<EntryB>().unwrap().message, "Hi!");

        anymap.insert(EntryA { value: 4 });
        assert_eq!(anymap.find::<EntryA>().unwrap().value, 4);

        anymap.remove::<EntryB>();
        assert!(anymap.find::<EntryB>().is_none());
    }

    #[test]
    fn anymap_iter() {
        let mut anymap = AnyMap::default();
        anymap.insert(EntryA { value: 3 });
        anymap.insert(EntryB {
            message: "Hi!".to_string(),
        });

        for (type_id, _value) in &anymap {
            println!("TypeId: {:?}", type_id);
        }
    }
}
