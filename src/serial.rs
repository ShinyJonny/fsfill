use std::marker::PhantomData;
use serde::ser::{Serialize, Serializer, SerializeTuple};
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor, Error};

/// Copiable, serializable, arbitraty-size array (slice).
#[derive(Clone, Copy, Debug)]
pub struct Array<T, const C: usize>(pub [T; C]);

// Implement default initialisation.
impl<T, const C: usize> Default for Array<T, C>
where
    T: Default + Copy
{
    fn default() -> Self
    {
        Self { 0: [T::default(); C] }
    }
}

// Implement serialisation with serde.
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

// Implement deserialisation with serde.
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
