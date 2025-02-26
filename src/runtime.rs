/*
 * @Author: jack cymqqqq@gmail.com
 * @Date: 2025-02-25 10:50:44
 * @LastEditors: jack cymqqqq@gmail.com
 * @LastEditTime: 2025-02-26 09:06:27
 * @FilePath: /bihelix-rgb-cli/src/runtime.rs
 * @Description: 这是默认设置,请设置`customMade`, 打开koroFileHeader查看配置 进行设置: https://github.com/OBKoro1/koro1FileHeader/wiki/%E9%85%8D%E7%BD%AE
 */
use super::*;

pub trait KVStored: Sized {
    fn load_bytes(bytes: &[u8]) -> Result<Self, DeserializeError>;
    fn store_bytes(&self) -> Result<Vec<u8>, SerializeError>;
}

impl KVStored for MemStash {
    fn load_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let confined_bytes = Confined::try_from_slice(bytes).unwrap();
        let de_mem = Self::from_strict_serialized::<U64>(confined_bytes)?;
        Ok(de_mem)
    }

    fn store_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let ser_bytes = self.to_strict_serialized::<U64>()?;
        Ok(ser_bytes.into_vec())
    }
}

impl KVStored for MemIndex {
    fn load_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let confined_bytes = Confined::try_from_slice(bytes).unwrap();
        let de_mem = Self::from_strict_serialized::<U64>(confined_bytes)?;
        Ok(de_mem)
    }

    fn store_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let ser_bytes = self.to_strict_serialized::<U64>()?;
        Ok(ser_bytes.into_vec())
    }
}

impl KVStored for MemState {
    fn load_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let confined_bytes = Confined::try_from_slice(bytes).unwrap();
        let de_mem = Self::from_strict_serialized::<U64>(confined_bytes)?;
        Ok(de_mem)
    }

    fn store_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let ser_bytes = self.to_strict_serialized::<U64>()?;

        Ok(ser_bytes.into_vec())
    }
}

pub trait KVStock: Sized {
    fn load_stock_bytes(
        stash_bytes: &[u8],
        index_bytes: &[u8],
        state_bytes: &[u8],
    ) -> Result<Self, DeserializeError>;
    fn store_stash_bytes(&mut self) -> Result<Vec<u8>, SerializeError>;
    fn store_state_bytes(&mut self) -> Result<Vec<u8>, SerializeError>;
    fn store_index_bytes(&mut self) -> Result<Vec<u8>, SerializeError>;
}

impl<S: StashProvider, H: StateProvider, P: IndexProvider> KVStock for Stock<S, H, P>
where
    S: KVStored,
    H: KVStored,
    P: KVStored,
{
    fn load_stock_bytes(
        stash_bytes: &[u8],
        index_bytes: &[u8],
        state_bytes: &[u8],
    ) -> Result<Self, DeserializeError> {
        let stash_provider = S::load_bytes(stash_bytes)?;
        let state_provider = H::load_bytes(state_bytes)?;
        let index_provider = P::load_bytes(index_bytes)?;
        Ok(Stock::with(stash_provider, state_provider, index_provider))
    }

    fn store_stash_bytes(&mut self) -> Result<Vec<u8>, SerializeError> {
        self.as_stash_provider_mut().store_bytes()
    }

    fn store_index_bytes(&mut self) -> Result<Vec<u8>, SerializeError> {
        self.as_index_provider_mut().store_bytes()
    }

    fn store_state_bytes(&mut self) -> Result<Vec<u8>, SerializeError> {
        self.as_state_provider_mut().store_bytes()
    }
}

#[derive(Getters)]
pub struct Runtime<
    S: StashProvider = MemStash,
    H: StateProvider = MemState,
    P: IndexProvider = MemIndex,
> where
    S: KVStored,
    H: KVStored,
    P: KVStored,
{
    database: String,
    pub stock: Stock<S, H, P>,
}

impl<S: StashProvider, H: StateProvider, P: IndexProvider> Runtime<S, H, P>
where
    S: KVStored,
    H: KVStored,
    P: KVStored,
{
    pub fn init_wallet_db(&mut self) -> Result<(), Error> {
        let db = RuntimeStore::new(self.stock_name()).unwrap();
        let stash_data = self
            .stock_mut()
            .store_stash_bytes()
            .map_err(|err| format!("Failed to get stash data: {:?}", err))
            .unwrap();
        let index_data = self
            .stock_mut()
            .store_index_bytes()
            .map_err(|err| format!("Failed to get index data: {:?}", err))
            .unwrap();
        let state_data = self
            .stock_mut()
            .store_state_bytes()
            .map_err(|err| format!("Failed to get state data: {:?}", err))
            .unwrap();
        db.init_db("STASH".as_bytes(), &stash_data).unwrap();
        db.init_db("INDEX".as_bytes(), &index_data).unwrap();
        db.init_db("STATE".as_bytes(), &state_data).unwrap();
        Ok(())
    }

    pub fn attach(name: &str, stock: Stock<S, H, P>) -> Self {
        Self {
            database: name.to_string(),
            stock,
        }
    }

    pub fn stock_mut(&mut self) -> &mut Stock<S, H, P> {
        &mut self.stock
    }

    pub fn stock_name(&self) -> &str {
        self.database.as_str()
    }
}

pub struct RuntimeStore {
    db: Arc<DB>,
}

impl RuntimeStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let db = DB::open_default(path)?;

        Ok(Self { db: Arc::new(db) })
    }
}

impl RuntimeStore {
    pub fn init_db(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.db.put(key, value).unwrap();
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        if let Some(db_value) = self.db.get(key)? {
            Ok(Some(db_value))
        } else {
            Ok(None)
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.db.put(key, value)?;
        Ok(())
    }
}
