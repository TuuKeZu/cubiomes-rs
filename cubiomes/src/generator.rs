use std::{
    i32,
    mem::{self, transmute},
};

use bitflags::bitflags;
use cubiomes_sys::{enums, getMinCacheSize, num_traits::FromPrimitive, Dimension, Range};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum GeneratorError {
    #[error("Biome id {0} is out of range and is not a valid biomeid")]
    BiomeIDOutOfRange(i32),
    #[error(
        "Function getBiomeAt failed (the ffi function returned -1), did you perhaps forgot to initialize the seed?"
    )]
    GetBiomeAtFailure,
    #[error("Function genBiomes failed (the ffi function did not return 0, it returne {0})")]
    GenBiomeToCacheFailure(i32),
    #[error("Index out of bounds")]
    IndexOutOfBound,
}

bitflags! {
    pub struct Flags: u32 {
        const LargeBiomes = 0x1;
        const NoBetaOcean = 0x2;
        const ForceOceanVariants = 0x4;

        //the source may set any bits
        const _ = !0;
    }
}

pub enum Scale {
    Block = 1,
    Biome = 4,
}

pub struct Generator {
    generator: cubiomes_sys::Generator,
}

pub struct Cache<'a> {
    cache: Vec<i32>,
    range: cubiomes_sys::Range,
    generator: &'a Generator,
}

impl Generator {
    /// Initializes a new generator for the given minecraft version and flags
    pub fn new(version: enums::MCVersion, flags: Flags) -> Self {
        unsafe {
            let mut generator: cubiomes_sys::Generator = mem::zeroed();

            cubiomes_sys::setupGenerator(&mut generator, version as i32, flags.bits());
            Self {
                generator: generator,
            }
        }
    }

    /// Sets the seed for the generator
    /// Trying to generate something without first selecting a seed
    /// will result in the generation failing
    pub fn apply_seed(&mut self, dimension: Dimension, seed: i64) {
        unsafe {
            cubiomes_sys::applySeed(&mut self.generator, dimension.0, transmute(seed));
        }
    }

    /// Gets the biome at the specified coordinates and scale
    ///
    /// Returns a biomeid or then an error.
    /// For the most consitent results querying surface biomes
    /// you should use 256 as the y value (minecraft build limit)
    pub fn get_biome_at(
        &self,
        scale: Scale,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<enums::BiomeID, GeneratorError> {
        unsafe {
            match cubiomes_sys::getBiomeAt(&self.generator, scale as i32, x, y, z) {
                -1 => Err(GeneratorError::GetBiomeAtFailure),
                n => FromPrimitive::from_i32(n).ok_or(GeneratorError::BiomeIDOutOfRange(n)),
            }
        }
    }

    fn get_min_cache_size_from_range(&self, range: Range) -> usize {
        self.get_min_cache_size(range.scale, range.sx, range.sy, range.sz)
    }

    fn get_min_cache_size(&self, scale: i32, size_x: i32, size_y: i32, size_z: i32) -> usize {
        unsafe { getMinCacheSize(&self.generator, scale, size_x, size_y, size_z) }
    }

    ///Fills the provided cache from the generator
    ///
    /// # Safety
    /// The caller must guarantee, that the cache is able to contain the generated data.
    /// The best way to guarantee this, is to use a cache generated from this generator
    /// using the ``new_cache()`` function.
    unsafe fn generate_biomes_to_cache(&self, cache: &mut Cache) -> Result<(), GeneratorError> {
        let result_num =
            cubiomes_sys::genBiomes(&self.generator, cache.cache.as_mut_ptr(), cache.range);

        //We set the caches lenght to that which the cubiome docs state it should be
        cache
            .cache
            .set_len(self.get_min_cache_size_from_range(cache.range));

        if result_num != 0 {
            return Err(GeneratorError::GenBiomeToCacheFailure(result_num));
        }
        Ok(())
    }
}

impl<'a> Generator {
    pub fn new_cache(&'a self, range: cubiomes_sys::Range) -> Cache<'a> {
        let cache_size = self.get_min_cache_size_from_range(range);

        let cache = Vec::with_capacity(cache_size);

        Cache {
            cache,
            range,
            generator: self,
        }
    }
}

impl Cache<'_> {
    pub fn fill_cache(&mut self) -> Result<(), GeneratorError> {
        unsafe { self.generator.generate_biomes_to_cache(self) }
    }

    pub fn get_cache(&mut self) -> &Vec<i32> {
        &self.cache
    }

    pub fn get_biome_at(&self, x: i32, y: i32, z: i32) -> Result<enums::BiomeID, GeneratorError> {
        let raw_biomeid = *self
            .cache
            .get((y * self.range.sx * self.range.sz + z * self.range.sx + x) as usize)
            .ok_or(GeneratorError::IndexOutOfBound)?;

        enums::BiomeID::from_i32(raw_biomeid)
            .ok_or_else(|| GeneratorError::BiomeIDOutOfRange(raw_biomeid))
    }
}
