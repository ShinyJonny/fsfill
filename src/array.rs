use std::marker::PhantomData;
use serde::ser::{Serialize, Serializer, SerializeTuple};
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor, Error};

/// Copiable, serializable, arbitrary-size slice.
#[derive(Clone, Copy, Debug, Eq)]
pub struct Array<T, const C: usize>(pub [T; C]);

impl<T, const C: usize> Default for Array<T, C>
where
    T: Default + Copy
{
    fn default() -> Self
    {
        Self { 0: [T::default(); C] }
    }
}

impl<T, const C: usize> PartialEq for Array<T, C>
where
    T: PartialEq
{
    fn eq(&self, other: &Self) -> bool
    {
        self.0.eq(&other.0)
    }
}

impl<T, const C: usize> Serialize for Array<T, C>
where
    T: Serialize
{
    // Reference: https://docs.serde.rs/src/serde/de/impls.rs.html
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut seq = serializer.serialize_tuple(C)?;
        for elem in &self.0 {
            seq.serialize_element(elem)?;
        }

        seq.end()
    }
}

impl<'de, T, const C: usize> Deserialize<'de> for Array<T, C>
where
    T: Deserialize<'de> + Default + Copy
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        deserializer.deserialize_tuple(C, ArrayVisitor { marker: PhantomData })
    }
}

// Source: https://docs.serde.rs/src/serde/de/impls.rs.html
#[derive(Debug)]
struct ArrayVisitor<A> {
    marker: PhantomData<A>,
}

impl<'de, T, const C: usize> Visitor<'de> for ArrayVisitor<Array<T, C>>
where
    T: Deserialize<'de> + Default + Copy
{
    type Value = Array<T, C>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        // FIXME: no length specification.
        formatter.write_str("an array")
    }

    // Reference: https://docs.serde.rs/src/serde/de/impls.rs.html
    #[inline]
    fn visit_seq<A>(self, mut seq: A) ->Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>
    {
        let mut arr = Array { 0: [T::default(); C] };

        for i in 0..C {
            arr.0[i] = match seq.next_element()? {
                Some(v) => v,
                None => return Err(Error::invalid_length(i, &self)),
            }
        }

        Ok(arr)
    }
}

#[cfg(test)]
mod tests {
    use super::Array;

    #[test]
    fn array0_equal()
    {
        let arr1:  Array<u32, 0> = Array {0: []};

        assert_eq!(arr1, Array {0: []});
    }

    #[test]
    fn array1_equal()
    {
        let mut arr1:  Array<u32, 1> = Array {0: [0]};
        arr1.0[0] = 4;

        assert_eq!(arr1, Array {0: [4]});
    }

    #[test]
    #[should_panic]
    fn array1_not_equal()
    {
        let arr1:  Array<u32, 1> = Array {0: [0]};

        assert_eq!(arr1, Array {0: [4]});
    }

    #[test]
    fn array5_equal()
    {
        let mut arr1:  Array<u32, 5> = Array {0: [0, 2, 4, 5, 6]};
        arr1.0[0] = 4;

        assert_eq!(arr1, Array {0: [4, 2, 4, 5, 6]});
    }

    #[test]
    #[should_panic]
    fn array5_not_equal()
    {
        let arr1:  Array<u32, 5> = Array {0: [0, 2, 4, 5, 6]};

        assert_eq!(arr1, Array {0: [4, 2, 4, 5, 6]});
    }
}
