// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! Generational arena for stable handle-based storage.
//!
//! The arena reserves generation `0` as invalid and bumps generations on remove
//! so stale handles stop matching reused slots. Generation wrap is handled
//! without panicking to avoid taking down long-lived daemon sessions after many
//! reuse cycles.
//!
//! Invariants:
//! - a live handle is valid only when both the slot index and generation match
//! - free-list reuse preserves the slot index but changes the generation
//! - callers must treat missing entries as normal stale-handle results rather
//!   than assuming allocation order implies validity

enum Slot<T> {
    Occupied {
        generation: u32,
        data: T,
    },
    Free {
        generation: u32,
        next: Option<usize>,
    },
}

// O(1) storage mapping index and generation.
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free_head: Option<usize>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_head: None,
        }
    }

    pub fn insert(&mut self, value: T) -> (u32, u32) {
        if let Some(index) = self.free_head {
            if let Slot::Free { generation, next } = self.slots[index] {
                self.free_head = next;
                self.slots[index] = Slot::Occupied {
                    generation,
                    data: value,
                };
                (index as u32, generation)
            } else {
                unreachable!("Free list is corrupted");
            }
        } else {
            let index = self.slots.len();
            let generation = 1;
            self.slots.push(Slot::Occupied {
                generation,
                data: value,
            });
            (index as u32, generation)
        }
    }

    pub fn get(&self, index: u32, generation: u32) -> Option<&T> {
        if let Some(Slot::Occupied {
            generation: g,
            data,
        }) = self.slots.get(index as usize)
            && *g == generation
        {
            return Some(data);
        }
        None
    }

    pub fn get_mut(&mut self, index: u32, generation: u32) -> Option<&mut T> {
        if let Some(Slot::Occupied {
            generation: g,
            data,
        }) = self.slots.get_mut(index as usize)
            && *g == generation
        {
            return Some(data);
        }
        None
    }

    pub fn remove(&mut self, index: u32, generation: u32) -> Option<T> {
        let valid =
            if let Some(Slot::Occupied { generation: g, .. }) = self.slots.get(index as usize) {
                *g == generation
            } else {
                false
            };
        if valid {
            // Keep generation 0 reserved so stale handles remain invalid even if
            // a very long-lived slot eventually wraps after repeated reuse.
            let next_gen = generation.wrapping_add(1).max(1);
            let slot = std::mem::replace(
                &mut self.slots[index as usize],
                Slot::Free {
                    generation: next_gen,
                    next: self.free_head,
                },
            );
            self.free_head = Some(index as usize);
            if let Slot::Occupied { data, .. } = slot {
                return Some(data);
            } else {
                unreachable!();
            }
        }
        None
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (u32, u32, &mut T)> {
        self.slots.iter_mut().enumerate().filter_map(|(i, slot)| {
            if let Slot::Occupied { generation, data } = slot {
                Some((i as u32, *generation, data))
            } else {
                None
            }
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            if let Slot::Occupied { generation, data } = slot {
                Some((i as u32, *generation, data))
            } else {
                None
            }
        })
    }
}
