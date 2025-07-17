//! # MyWrapDatabaseAsync
//!
//! This module provides a fork of REVM's `WrapDatabaseAsync`,
//! with an additional `get_db_mut` method for advanced control.
//!
//! ## Key Features
//!
//! - Wraps any `DatabaseAsync` or `DatabaseAsyncRef` to provide a synchronous `Database`/`DatabaseRef` interface.
//! - Adds `get_db_mut`, allowing direct mutable access to the underlying async database.
//!   This is especially useful for operations such as resetting the block number or other configuration/state changes
//!   that are not covered by the standard trait interfaces.
//! - Maintains compatibility with both async and sync REVM database traits.
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! let mut wrapped_db = MyWrapDatabaseAsync::new(alloy_db)?;
//! let inner_db = wrapped_db.get_db_mut();
//! inner_db.set_block_number(new_block_id);
//! ```
//!
//! This extension is essential for scenarios where you need to update the underlying database state
//! (e.g., switching block context) without reconstructing the entire wrapper.

use revm::{
    database::{Database, DatabaseRef},
    database_interface::async_db::{DatabaseAsync, DatabaseAsyncRef},
    primitives::{Address, StorageKey, StorageValue, B256},
    state::{AccountInfo, Bytecode},
};

use core::future::Future;
use tokio::runtime::{Handle, Runtime};

/// Wraps a [DatabaseAsync] or [DatabaseAsyncRef] to provide a [`Database`] implementation.
#[derive(Debug)]
pub struct MyWrapDatabaseAsync<T> {
    db: T,
    rt: HandleOrRuntime,
}

impl<T> MyWrapDatabaseAsync<T> {
    /// Wraps a [DatabaseAsync] or [DatabaseAsyncRef] instance.
    ///
    /// Returns `None` if no tokio runtime is available or if the current runtime is a current-thread runtime.
    pub fn new(db: T) -> Option<Self> {
        let rt = match Handle::try_current() {
            Ok(handle) => match handle.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::CurrentThread => return None,
                _ => HandleOrRuntime::Handle(handle),
            },
            Err(_) => return None,
        };
        Some(Self { db, rt })
    }

    /// Gets a mutable reference to the inner database
    ///
    /// This allows direct access to the wrapped database instance for operations
    /// that are not covered by the standard Database/DatabaseRef interfaces,
    /// such as configuration changes or state updates.
    ///
    /// # Example
    /// ```rust,ignore
    /// let mut wrapped_db = MyWrapDatabaseAsync::new(alloy_db)?;
    /// let inner_db = wrapped_db.get_db_mut();
    /// inner_db.set_block_number(new_block_id);
    /// ```
    pub fn get_db_mut(&mut self) -> &mut T {
        &mut self.db
    }

    /// Wraps a [DatabaseAsync] or [DatabaseAsyncRef] instance, with a runtime.
    ///
    /// Refer to [tokio::runtime::Builder] on how to create a runtime if you are in synchronous world.
    ///
    /// If you are already using something like [tokio::main], call [`WrapDatabaseAsync::new`] instead.
    pub fn with_runtime(db: T, runtime: Runtime) -> Self {
        let rt = HandleOrRuntime::Runtime(runtime);
        Self { db, rt }
    }

    /// Wraps a [DatabaseAsync] or [DatabaseAsyncRef] instance, with a runtime handle.
    ///
    /// This generally allows you to pass any valid runtime handle, refer to [tokio::runtime::Handle] on how
    /// to obtain a handle.
    ///
    /// If you are already in asynchronous world, like [tokio::main], use [`WrapDatabaseAsync::new`] instead.
    pub fn with_handle(db: T, handle: Handle) -> Self {
        let rt = HandleOrRuntime::Handle(handle);
        Self { db, rt }
    }
}

impl<T: DatabaseAsync> Database for MyWrapDatabaseAsync<T> {
    type Error = T::Error;

    #[inline]
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.rt.block_on(self.db.basic_async(address))
    }

    #[inline]
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.rt.block_on(self.db.code_by_hash_async(code_hash))
    }

    #[inline]
    fn storage(
        &mut self,
        address: Address,
        index: StorageKey,
    ) -> Result<StorageValue, Self::Error> {
        self.rt.block_on(self.db.storage_async(address, index))
    }

    #[inline]
    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        self.rt.block_on(self.db.block_hash_async(number))
    }
}

impl<T: DatabaseAsyncRef> DatabaseRef for MyWrapDatabaseAsync<T> {
    type Error = T::Error;

    #[inline]
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.rt.block_on(self.db.basic_async_ref(address))
    }

    #[inline]
    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.rt.block_on(self.db.code_by_hash_async_ref(code_hash))
    }

    #[inline]
    fn storage_ref(
        &self,
        address: Address,
        index: StorageKey,
    ) -> Result<StorageValue, Self::Error> {
        self.rt.block_on(self.db.storage_async_ref(address, index))
    }

    #[inline]
    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        self.rt.block_on(self.db.block_hash_async_ref(number))
    }
}

// Hold a tokio runtime handle or full runtime
#[derive(Debug)]
enum HandleOrRuntime {
    Handle(Handle),
    Runtime(Runtime),
}

impl HandleOrRuntime {
    #[inline]
    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + Send,
        F::Output: Send,
    {
        match self {
            Self::Handle(handle) => tokio::task::block_in_place(move || handle.block_on(f)),
            Self::Runtime(rt) => rt.block_on(f),
        }
    }
}
