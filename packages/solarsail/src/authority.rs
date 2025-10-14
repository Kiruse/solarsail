use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, Storage as StdStorage};
pub use cw_utils::Expiration;
use solarsail_macros::emit;
use thiserror::Error;

use crate::ExecuteContext;

pub type Storage = cw_storage_plus::Item<AuthorityState>;

#[cw_serde]
pub struct AuthorityState {
  pub addr: Addr,
  pub transfer: Option<AuthorityTransferState>,
}

#[cw_serde]
pub struct AuthorityTransferState {
  pub addr: Addr,
  pub expires: Expiration,
}

pub trait Authority {
  /// Low level: Get the underlying storage item of this authority.
  fn storage(&self) -> Storage;

  /// Get the display name of this authority (for events).
  fn name(&self) -> &str;

  /// Get this authority's current address from storage.
  fn get(&self, storage: &dyn StdStorage) -> Result<Addr, AuthorityError> {
    Ok(self.storage().load(storage)?.addr)
  }

  /// Attempt to get this authority's current address from storage. If not set, returns `None`. If
  /// failed to parse, returns an `Err`.
  fn get_maybe(&self, storage: &dyn StdStorage) -> Result<Option<Addr>, AuthorityError> {
    Ok(self.storage().may_load(storage)?.map(|s| s.addr))
  }

  /// Check if the current sender is the authority.
  fn check(&self, ctx: &ExecuteContext) -> Result<(), AuthorityError> {
    if self.get(ctx.deps.storage)? == ctx.info.sender {
      Ok(())
    } else {
      Err(AuthorityError::Unauthorized)
    }
  }

  /// Check if the current sender is the pending authority.
  fn check_transfer_to(&self, ctx: &ExecuteContext) -> Result<(), AuthorityError> {
    let state = self.storage().load(ctx.deps.storage)?;
    state.transfer
      .map(|transfer| transfer.addr == ctx.info.sender && !transfer.expires.is_expired(&ctx.env.block))
      .ok_or(AuthorityError::NotFound)?;
    Ok(())
  }

  /// Initiate transfer of authority. Must be confirmed by the new authority. Until confirmed, the
  /// current authority can still transfer to another address, overriding this transfer.
  fn transfer(&self, ctx: &mut ExecuteContext, new_authority: Option<Addr>, expires: Expiration) -> Result<(), AuthorityError> {
    self.check(ctx)?;
    match new_authority {
      Some(new_authority) => {
        let store = self.storage();
        let mut state = store.load(ctx.deps.storage)?;
        state.transfer = Some(AuthorityTransferState {
          addr: new_authority.clone(),
          expires,
        });
        store.save(ctx.deps.storage, &state)?;

        emit!("authority.begin_transfer", {
          authority: self.name(),
          address: new_authority,
          expires: expires.to_string(),
        });
        Ok(())
      }
      None => {
        self.storage().remove(ctx.deps.storage);
        emit!("authority.renounce", { authority: self.name() });
        Ok(())
      }
    }
  }

  /// Accept a transfer of authority. Must be initiated by the current authority with [`Self::transfer`].
  fn accept_transfer(&self, ctx: &mut ExecuteContext) -> Result<(), AuthorityError> {
    self.check_transfer_to(ctx)?;
    let store = self.storage();
    let mut state = store.load(ctx.deps.storage)?;
    state.addr = state.transfer.take().unwrap().addr;
    state.transfer = None;
    store.save(ctx.deps.storage, &state)?;
    emit!("authority.accept_transfer", { authority: self.name(), address: state.addr });
    Ok(())
  }
}

pub trait AuthorityTransfer {
  type Authority: Authority;

  /// Get the address to transfer the authority to.
  fn addr(&self) -> &Option<Addr>;

  /// Get the expiration time of the transfer.
  fn expires(&self) -> &Expiration;

  /// Get the authority to transfer the authority to.
  fn authority(&self) -> Self::Authority;

  /// Transfer the authority to the address. Convenience wrapper for [`Authority::transfer`].
  fn transfer(&self, ctx: &mut ExecuteContext) -> Result<(), AuthorityError> where Self: Sized {
    self.authority().transfer(ctx, self.addr().clone(), self.expires().clone())
  }

  /// Accept the transfer of authority. Convenience wrapper for [`Authority::accept_transfer`].
  fn accept(&self, ctx: &mut ExecuteContext) -> Result<(), AuthorityError> where Self: Sized {
    self.authority().accept_transfer(ctx)
  }
}

#[derive(Error, Debug)]
pub enum AuthorityError {
  #[error("Unauthorized")]
  Unauthorized,

  #[error("NotFound")]
  NotFound,

  #[error("{0}")]
  Std(#[from] StdError),
}
