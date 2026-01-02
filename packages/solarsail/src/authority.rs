use cosmwasm_std::{Addr, Event, StdError, Storage as StdStorage};
pub use cw_utils::Expiration;
use solarsail_macros::solarize;
use thiserror::Error;

use crate::ExecuteContext;
use crate::context::ExecuteContextDetails;

use crate as solarsail;

pub type Storage = cw_storage_plus::Item<AuthorityState>;

#[solarize]
pub struct AuthorityState {
  pub addr: Addr,
  pub transfer: Option<AuthorityTransferState>,
}

#[solarize]
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
    if self.get(ctx.deps().storage)? == ctx.info().sender {
      Ok(())
    } else {
      Err(AuthorityError::Unauthorized)
    }
  }

  /// Check if the current sender is the pending authority.
  fn check_transfer_to(&self, ctx: &ExecuteContext) -> Result<(), AuthorityError> {
    let state = self.storage().load(ctx.deps().storage)?;
    state.transfer
      .map(|transfer| transfer.addr == ctx.info().sender && !transfer.expires.is_expired(&ctx.env().block))
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
        let mut state = store.load(ctx.deps().storage)?;
        state.transfer = Some(AuthorityTransferState {
          addr: new_authority.clone(),
          expires,
        });
        store.save(ctx.deps_mut().storage, &state)?;

        let event = Event::new("authority.begin_transfer")
          .add_attributes([
            ("authority", self.name()),
            ("address", &new_authority.to_string()),
            ("expires", &expires.to_string()),
          ]);
        ctx.emit(event);
        Ok(())
      }
      None => {
        self.storage().remove(ctx.deps_mut().storage);
        let event = Event::new("authority.renounce")
          .add_attributes([
            ("authority", self.name()),
          ]);
        ctx.emit(event);
        Ok(())
      }
    }
  }

  /// Accept a transfer of authority. Must be initiated by the current authority with [`Self::transfer`].
  fn accept_transfer(&self, ctx: &mut ExecuteContext) -> Result<(), AuthorityError> {
    self.check_transfer_to(ctx)?;
    let store = self.storage();
    let mut state = store.load(ctx.deps_mut().storage)?;
    state.addr = state.transfer.take().unwrap().addr;
    state.transfer = None;
    store.save(ctx.deps_mut().storage, &state)?;
    let event = Event::new("authority.accept_transfer")
      .add_attributes([
        ("authority", self.name()),
        ("address", &state.addr.to_string()),
      ]);
    ctx.emit(event);
    Ok(())
  }

  /// Admin override of authority.
  fn assign(&self, ctx: &mut ExecuteContext, new_authority: Option<Addr>) -> Result<(), AuthorityError> {
    match new_authority {
      Some(new_authority) => {
        let store = self.storage();
        let mut state = store.load(ctx.deps_mut().storage)?;
        state.addr = new_authority.clone();
        store.save(ctx.deps_mut().storage, &state)?;
        let event = Event::new("authority.override")
          .add_attributes([
            ("authority", self.name()),
            ("address", &new_authority.to_string()),
          ]);
        ctx.emit(event);
        Ok(())
      }
      None => {
        self.storage().remove(ctx.deps_mut().storage);
        let event = Event::new("authority.renounce")
          .add_attributes([
            ("authority", self.name()),
          ]);
        ctx.emit(event);
        Ok(())
      }
    }
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
