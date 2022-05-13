use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

use serde::Serialize;


const MIN_CAPACITY: usize = 8200;


/// Data structure for tracking free/used space on a drive.
#[derive(Clone, Debug, Serialize)]
pub struct UsageMap(pub Vec<Segment>);

impl UsageMap {
    /// Creates a new UsageMap with the specified size.
    pub fn new(len: u64) -> Self
    {
        assert!(len > 0);

        // TODO: implement better capacity prediction.
        let capacity = usize::max(
            MIN_CAPACITY,
            len as usize / 30000
        );

        let mut vec = Vec::with_capacity(capacity);
        vec.push(
            Segment {
                start: 0,
                end: len,
                status: AllocStatus::Free,
            }
        );

        Self { 0: vec }
    }

    /// Returns the number of segments in the map.
    pub fn len(&self) -> usize
    {
        self.0.len()
    }

    /// Returns the size of the map, i.e. the max address.
    pub fn size(&self) -> u64
    {
        self.0.last().unwrap().end
    }

    /// Updates a portion of the map.
    /// The method tolerates size reaching beyond the end of the map, and cuts it off. Updating the
    /// map with the start reaching beyond the end of the map will panic.
    pub fn update(&mut self, start: u64, size: u64, status: AllocStatus)
    {
        // Tolerate reaching beyond the end of the map.
        let map_size = self.size();
        let end = if start + size > map_size {
            map_size
        } else {
            start + size
        };

        assert!(start <= map_size);

        self.add_segment(Segment { start, end, status });
    }

    /// Update a portion of the map with a raw Segment.
    pub fn add_segment(&mut self, new: Segment)
    {
        let vector = &self.0;

        if new.start == new.end { return; }
        assert!(new.start < new.end);
        assert!(new.end <= vector.iter().last().unwrap().end);

        // Get the indices of the nodes within which the new segment's start and end are.

        let start_i = vector.iter().position(|e| {
            new.start >= e.start && new.start < e.end
        }).unwrap();
        let mut end_i = vector.iter().position(|e| {
            new.end > e.start && new.end <= e.end
        }).unwrap();

        let vector = &mut self.0;

        // Delete all the segments in-between the start and end segments.
        for _ in (start_i + 1)..end_i {
            vector.remove(start_i + 1);
        }

        // If the start and the end are in one segment, duplicate the segment for consistency.
        if start_i == end_i {
            vector.insert(start_i + 1, vector[start_i]);
        }

        end_i = start_i + 1;

        if vector[start_i].status == vector[end_i].status {
            if vector[start_i].status == new.status {
                vector[start_i].end = vector[end_i].end;
                vector.remove(end_i);
            } else {
                vector[start_i].end = new.start;
                vector[end_i].start = new.end;
                vector.insert(start_i + 1, new);
            }
        } else {
            if vector[start_i].status == new.status {
                vector[start_i].end = new.end;
                vector[end_i].start = new.end;
            } else {
                vector[start_i].end = new.start;
                vector[end_i].start = new.start;
            }
        }

        // Remove remaining zero-sized segments and merge neighbours of the same status.

        self.clean_zero_sized();
        self.merge_neighbours();
    }

    /// Remove zero-sized segments.
    fn clean_zero_sized(&mut self)
    {
        while let Some(pos) = self.0.iter()
            .position(|e| { e.start == e.end })
        {
            self.0.remove(pos);
        }
    }

    /// Merge neighbouring segments of the same type.
    fn merge_neighbours(&mut self)
    {
        let vector = &mut self.0;
        let mut head = 0;

        loop {
            if head + 1 >= vector.len() {
                break;
            }

            if vector[head].status == vector[head + 1].status {
                vector[head].end = vector[head + 1].end;
                vector.remove(head + 1);
            } else {
                head += 1;
            }
        }
    }
}


/// Data structure representing a run of bytes on a drive.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Segment {
    pub start: u64,
    pub end: u64,
    pub status: AllocStatus,
}

impl Segment {
    pub fn size(&self) -> usize
    {
        (self.end - self.start) as usize
    }
}


/// Allocation status of a Segment.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum AllocStatus {
    Free,
    Used,
}


// Trait implementations.


// Iterating.

impl<'a> IntoIterator for UsageMap {
    type Item = Segment;
    type IntoIter = <Vec<Segment> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter
    {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a UsageMap {
    type Item = &'a Segment;
    type IntoIter = <&'a Vec<Segment> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter
    {
        self.0.as_slice().into_iter()
    }
}

impl<'a> IntoIterator for &'a mut UsageMap {
    type Item = &'a mut Segment;
    type IntoIter = <&'a mut Vec<Segment> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter
    {
        self.0.as_mut_slice().into_iter()
    }
}

// Indexing

impl<I> Index<I> for UsageMap
where
    I: SliceIndex<[Segment]>
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output
    {
        &self.0[index]
    }
}

impl<I> IndexMut<I> for UsageMap
where
    I: SliceIndex<[Segment]>
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output
    {
        &mut self.0[index]
    }
}


// Tests.


#[cfg(test)]
mod tests {
    use super::{AllocStatus, UsageMap, Segment};

    mod usage_map {
        use super::*;

        // NOTE: tests were not done for:
        //  * UsageMap for IntoIterator.
        //  * &UsageMap for IntoIterator.
        //  * &mut UsageMap for IntoIterator.
        //  * UsageMap for Index.
        //  * UsageMap for IndexMut.
        //
        //  * UsageMap::size().

        #[test]
        fn new()
        {
            let map = UsageMap::new(5);

            assert_eq!(map[0], Segment { start: 0, end: 5, status: AllocStatus::Free });
        }

        #[test]
        #[should_panic]
        fn new_zero_size()
        {
            UsageMap::new(0);
        }

        #[test]
        fn len_1()
        {
            let map = UsageMap::new(5);

            assert_eq!(map.len(), 1);
        }

        #[test]
        fn len_2()
        {
            let mut map = UsageMap::new(5);
            map.add_segment(Segment { start: 2, end: 5, status: AllocStatus::Used });

            assert_eq!(map.len(), 2);
        }

        #[test]
        fn add_segment_start_eq_end()
        {
            let mut map = UsageMap::new(5);
            let orig_e = map[0];

            map.add_segment(Segment { start: 1, end: 1, status: AllocStatus::Used });

            assert_eq!(map.len(), 1);
            assert_eq!(map[0], orig_e);
        }

        #[test]
        #[should_panic]
        fn add_segment_start_gt_end()
        {
            let mut map = UsageMap::new(5);
            map.add_segment(Segment { start: 3, end: 1, status: AllocStatus::Used });
        }

        #[test]
        #[should_panic]
        fn add_segment_end_out_of_bounds()
        {
            let mut map = UsageMap::new(5);
            map.add_segment(Segment { start: 0, end: 6, status: AllocStatus::Used });
        }

        #[test]
        fn add_segment_inside_one_different_status()
        {
            let mut map = UsageMap::new(20);
            let new_segment = Segment { start: 2, end: 11, status: AllocStatus::Used };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 3);
            assert_eq!(map[0], Segment { start: 0, end: 2, status: AllocStatus::Free });
            assert_eq!(map[1], new_segment);
            assert_eq!(map[2], Segment { start: 11, end: 20, status: AllocStatus::Free });
        }

        #[test]
        fn add_segment_inside_one_same_status()
        {
            let mut map = UsageMap::new(20);
            let new_segment = Segment { start: 2, end: 11, status: AllocStatus::Free };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 1);
            assert_eq!(map[0].start, 0);
            assert_eq!(map[0].end, 20);
            assert_eq!(map[0].status, AllocStatus::Free);
        }

        #[test]
        fn add_segment_inside_two_different_first_same()
        {
            let mut map = UsageMap::new(20);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            let new_segment = Segment { start: 3, end: 11, status: AllocStatus::Free };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 2);
            assert_eq!(map[0].start, 0);
            assert_eq!(map[0].end, 11);
            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].start, 11);
            assert_eq!(map[1].end, 20);
            assert_eq!(map[1].status, AllocStatus::Used);
        }

        #[test]
        fn add_segment_inside_two_different_second_same()
        {
            let mut map = UsageMap::new(20);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            let new_segment = Segment { start: 5, end: 16, status: AllocStatus::Used };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 2);
            assert_eq!(map[0].start, 0);
            assert_eq!(map[0].end, 5);
            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].start, 5);
            assert_eq!(map[1].end, 20);
            assert_eq!(map[1].status, AllocStatus::Used);
        }

        #[test]
        fn add_segment_add_a_bunch()
        {
            let mut map = UsageMap::new(100);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            map.add_segment(Segment { start: 30, end: 40, status: AllocStatus::Used });
            map.add_segment(Segment { start: 40, end: 50, status: AllocStatus::Free });
            map.add_segment(Segment { start: 50, end: 60, status: AllocStatus::Used });

            assert_eq!(map.len(), 7);

            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].status, AllocStatus::Used);
            assert_eq!(map[2].status, AllocStatus::Free);
            assert_eq!(map[3].status, AllocStatus::Used);
            assert_eq!(map[4].status, AllocStatus::Free);
            assert_eq!(map[5].status, AllocStatus::Used);
            assert_eq!(map[6].status, AllocStatus::Free);

            assert_eq!(map[0].start, 0);
            assert_eq!(map[1].start, 10);
            assert_eq!(map[2].start, 20);
            assert_eq!(map[3].start, 30);
            assert_eq!(map[4].start, 40);
            assert_eq!(map[5].start, 50);
            assert_eq!(map[6].start, 60);

            assert_eq!(map[0].end, 10);
            assert_eq!(map[1].end, 20);
            assert_eq!(map[2].end, 30);
            assert_eq!(map[3].end, 40);
            assert_eq!(map[4].end, 50);
            assert_eq!(map[5].end, 60);
            assert_eq!(map[6].end, 100);
        }

        #[test]
        fn add_segment_span_first_status_same()
        {
            let mut map = UsageMap::new(100);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            map.add_segment(Segment { start: 30, end: 40, status: AllocStatus::Used });
            map.add_segment(Segment { start: 40, end: 50, status: AllocStatus::Free });
            map.add_segment(Segment { start: 50, end: 60, status: AllocStatus::Used });
            let new_segment = Segment { start: 15, end: 45, status: AllocStatus::Used };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 5);

            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].status, AllocStatus::Used);
            assert_eq!(map[2].status, AllocStatus::Free);
            assert_eq!(map[3].status, AllocStatus::Used);
            assert_eq!(map[4].status, AllocStatus::Free);

            assert_eq!(map[0].start, 0);
            assert_eq!(map[1].start, 10);
            assert_eq!(map[2].start, 45);
            assert_eq!(map[3].start, 50);
            assert_eq!(map[4].start, 60);

            assert_eq!(map[0].end, 10);
            assert_eq!(map[1].end, 45);
            assert_eq!(map[2].end, 50);
            assert_eq!(map[3].end, 60);
            assert_eq!(map[4].end, 100);
        }

        #[test]
        fn add_segment_span_last_status_same()
        {
            let mut map = UsageMap::new(100);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            map.add_segment(Segment { start: 30, end: 40, status: AllocStatus::Used });
            map.add_segment(Segment { start: 40, end: 50, status: AllocStatus::Free });
            map.add_segment(Segment { start: 50, end: 60, status: AllocStatus::Used });
            let new_segment = Segment { start: 15, end: 45, status: AllocStatus::Free };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 5);

            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].status, AllocStatus::Used);
            assert_eq!(map[2].status, AllocStatus::Free);
            assert_eq!(map[3].status, AllocStatus::Used);
            assert_eq!(map[4].status, AllocStatus::Free);

            assert_eq!(map[0].start, 0);
            assert_eq!(map[1].start, 10);
            assert_eq!(map[2].start, 15);
            assert_eq!(map[3].start, 50);
            assert_eq!(map[4].start, 60);

            assert_eq!(map[0].end, 10);
            assert_eq!(map[1].end, 15);
            assert_eq!(map[2].end, 50);
            assert_eq!(map[3].end, 60);
            assert_eq!(map[4].end, 100);
        }

        #[test]
        fn add_segment_starts_at_boundary_same()
        {
            let mut map = UsageMap::new(40);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            map.add_segment(Segment { start: 30, end: 40, status: AllocStatus::Used });
            let new_segment = Segment { start: 10, end: 25, status: AllocStatus::Used };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 4);

            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].status, AllocStatus::Used);
            assert_eq!(map[2].status, AllocStatus::Free);
            assert_eq!(map[3].status, AllocStatus::Used);

            assert_eq!(map[0].start, 0);
            assert_eq!(map[1].start, 10);
            assert_eq!(map[2].start, 25);
            assert_eq!(map[3].start, 30);

            assert_eq!(map[0].end, 10);
            assert_eq!(map[1].end, 25);
            assert_eq!(map[2].end, 30);
            assert_eq!(map[3].end, 40);
        }

        #[test]
        fn add_segment_starts_at_boundary_different()
        {
            let mut map = UsageMap::new(40);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            map.add_segment(Segment { start: 30, end: 40, status: AllocStatus::Used });
            let new_segment = Segment { start: 10, end: 25, status: AllocStatus::Free };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 2);

            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].status, AllocStatus::Used);

            assert_eq!(map[0].start, 0);
            assert_eq!(map[1].start, 30);

            assert_eq!(map[0].end, 30);
            assert_eq!(map[1].end, 40);
        }

        #[test]
        fn add_segment_ends_at_boundary_same()
        {
            let mut map = UsageMap::new(40);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            map.add_segment(Segment { start: 30, end: 40, status: AllocStatus::Used });
            let new_segment = Segment { start: 15, end: 30, status: AllocStatus::Free };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 4);

            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].status, AllocStatus::Used);
            assert_eq!(map[2].status, AllocStatus::Free);
            assert_eq!(map[3].status, AllocStatus::Used);

            assert_eq!(map[0].start, 0);
            assert_eq!(map[1].start, 10);
            assert_eq!(map[2].start, 15);
            assert_eq!(map[3].start, 30);

            assert_eq!(map[0].end, 10);
            assert_eq!(map[1].end, 15);
            assert_eq!(map[2].end, 30);
            assert_eq!(map[3].end, 40);
        }

        #[test]
        fn add_segment_ends_at_boundary_different()
        {
            let mut map = UsageMap::new(40);
            map.add_segment(Segment { start: 10, end: 20, status: AllocStatus::Used });
            map.add_segment(Segment { start: 30, end: 40, status: AllocStatus::Used });
            let new_segment = Segment { start: 15, end: 30, status: AllocStatus::Used };
            map.add_segment(new_segment);

            assert_eq!(map.len(), 2);

            assert_eq!(map[0].status, AllocStatus::Free);
            assert_eq!(map[1].status, AllocStatus::Used);

            assert_eq!(map[0].start, 0);
            assert_eq!(map[1].start, 10);

            assert_eq!(map[0].end, 10);
            assert_eq!(map[1].end, 40);
        }
    }


    mod segment {
        use super::*;

        #[test]
        fn eq()
        {
            let e1 = Segment { start: 0, end: 0, status: AllocStatus::Free};
            assert_eq!(e1, Segment { start: 0, end: 0, status: AllocStatus::Free});

            let e1 = Segment { start: 10, end: 0, status: AllocStatus::Free};
            assert_eq!(e1, Segment { start: 10, end: 0, status: AllocStatus::Free});

            let e1 = Segment { start: 3, end: 20, status: AllocStatus::Free};
            assert_eq!(e1, Segment { start: 3, end: 20, status: AllocStatus::Free});

            let e1 = Segment { start: 55, end: 300, status: AllocStatus::Used};
            assert_eq!(e1, Segment { start: 55, end: 300, status: AllocStatus::Used});
        }

        #[test]
        #[should_panic]
        fn start_not_eq()
        {
            let e1 = Segment { start: 1, end: 0, status: AllocStatus::Free};
            assert_eq!(e1, Segment { start: 0, end: 0, status: AllocStatus::Free});
        }

        #[test]
        #[should_panic]
        fn end_not_eq()
        {
            let e1 = Segment { start: 0, end: 1, status: AllocStatus::Free};
            assert_eq!(e1, Segment { start: 0, end: 0, status: AllocStatus::Free});
        }

        #[test]
        #[should_panic]
        fn status_not_eq()
        {
            let e1 = Segment { start: 0, end: 0, status: AllocStatus::Used};
            assert_eq!(e1, Segment { start: 0, end: 0, status: AllocStatus::Free});
        }
    }


    mod alloc_status {
        use super::*;

        #[test]
        fn eq()
        {
            assert_eq!(AllocStatus::Free, AllocStatus::Free);
            assert_eq!(AllocStatus::Used, AllocStatus::Used);
        }

        #[test]
        #[should_panic]
        fn not_eq()
        {
            assert_eq!(AllocStatus::Used, AllocStatus::Free);
        }
    }
}
