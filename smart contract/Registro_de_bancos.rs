#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use stylus_sdk::{
    alloy_primitives::Address,
    prelude::*,
    storage::{StorageBool, StorageMap, StorageOwner},
};

// Define el almacenamiento del contrato
#[sol_storage]
#[entrypoint]
pub struct BankRegistry {
    /// El dueño del contrato (quien puede agregar bancos)
    owner: StorageOwner,

    /// Mapping de direcciones de bancos a un booleano (true si es confiable)
    trusted_banks: StorageMap<Address, StorageBool>,
}

// Define los métodos externos (la ABI)
#[external]
impl BankRegistry {
    /// Función de "solo-dueño" para agregar un nuevo banco a la whitelist
    pub fn add_bank(&mut self, bank_address: Address) -> Result<(), Vec<u8>> {
        // Verifica que quien llama es el dueño del contrato
        self.owner.guard()?;
        
        // Agrega el banco
        self.trusted_banks.insert(bank_address, true);
        Ok(())
    }

    /// Función de "solo-dueño" para remover un banco
    pub fn remove_bank(&mut self, bank_address: Address) -> Result<(), Vec<u8>> {
        self.owner.guard()?;
        
        // Remueve el banco
        self.trusted_banks.insert(bank_address, false);
        Ok(())
    }

    /// Función de solo-lectura para verificar si un banco es confiable
    #[view]
    pub fn is_trusted_bank(&self, bank_address: Address) -> Result<bool, Vec<u8>> {
        Ok(self.trusted_banks.get(bank_address))
    }

    /// Función para transferir la propiedad del contrato
    pub fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), Vec<u8>> {
        self.owner.guard()?;
        self.owner.set(new_owner);
        Ok(())
    }
}