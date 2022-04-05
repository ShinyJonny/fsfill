#![allow(dead_code)]
use std::io::Read;


/// Lightweight bitmap abstraction.
pub struct Bitmap(Vec<u8>);

impl Bitmap {
    pub fn from_bytes(bytes: &[u8]) -> Self
    {
        Self { 0: bytes.to_vec() }
    }

    pub fn from_reader<R: Read>(reader: &mut R, size: usize) -> Result<Self, std::io::Error>
    {
        let mut vec = vec![u8::default(); size];
        reader.read_exact(vec.as_mut_slice())?;

        Ok(Self { 0: vec })
    }

    pub fn check_bit(&self, idx: usize) -> bool
    {
        let byte = self.0[idx / 8];
        let bit_pos = (8 - 1) - idx % 8;

        if (byte >> bit_pos) & 0x01 == 1 {
            true
        } else {
            false
        }
    }
}


#[cfg(test)]
mod tests {
    use super::Bitmap;

    #[test]
    fn check_bit()
    {
        let bmp = Bitmap::from_bytes(&[0x43, 0x56, 0xfa]);

        assert_eq!(false, bmp.check_bit(0));
        assert_eq!(true,  bmp.check_bit(1));
        assert_eq!(false, bmp.check_bit(2));
        assert_eq!(false, bmp.check_bit(3));
        assert_eq!(false, bmp.check_bit(4));
        assert_eq!(false, bmp.check_bit(5));
        assert_eq!(true,  bmp.check_bit(6));
        assert_eq!(true,  bmp.check_bit(7));

        assert_eq!(false, bmp.check_bit(8));
        assert_eq!(true,  bmp.check_bit(9));
        assert_eq!(false, bmp.check_bit(10));
        assert_eq!(true,  bmp.check_bit(11));
        assert_eq!(false, bmp.check_bit(12));
        assert_eq!(true,  bmp.check_bit(13));
        assert_eq!(true,  bmp.check_bit(14));
        assert_eq!(false, bmp.check_bit(15));

        assert_eq!(true,  bmp.check_bit(16));
        assert_eq!(true,  bmp.check_bit(17));
        assert_eq!(true,  bmp.check_bit(18));
        assert_eq!(true,  bmp.check_bit(19));
        assert_eq!(true,  bmp.check_bit(20));
        assert_eq!(false, bmp.check_bit(21));
        assert_eq!(true,  bmp.check_bit(22));
        assert_eq!(false, bmp.check_bit(23));
    }
}
