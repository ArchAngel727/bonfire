// src/lib/crypto.ts
//
// Thin wrapper around the Tauri crypto_* commands. Vec<u8> on the Rust
// side shows up as number[] here, matching the rest of the codebase's
// wire format and the server's bundle shapes exactly.

import { invoke } from '@tauri-apps/api/core'; // Tauri 2.x; 1.x: '@tauri-apps/api/tauri'

export interface Identity {
  curve25519: number[];
  ed25519: number[];
  keys_published: boolean;
}

export interface OneTimePrekey {
  prekey_id: number;
  public_key: number[];
}

/** Shape matches the server's `UploadBundleRequest`. */
export interface BundleUpload {
  identity_key: number[];
  signing_key: number[];
  signed_prekey: number[];
  signed_prekey_signature: number[];
  signed_prekey_id: number;
  one_time_prekeys: OneTimePrekey[];
}

export interface EncryptedMessage {
  /** 0 = pre-key message, 1 = normal message (libolm convention) */
  message_type: number;
  ciphertext: number[];
}

/** Load (or create) crypto state for this user. Each user has their own
 *  subdirectory under <app_data>/crypto/<user_id>/, so logging out and
 *  back in as a different user doesn't disturb the previous user's keys. */
export const attachCrypto = (userId: string): Promise<void> =>
  invoke('crypto_attach', { userId });

/** Drop the in-memory state. On-disk state is preserved. Call on logout. */
export const detachCrypto = (): Promise<void> => invoke('crypto_detach');

/** Wipe the current user's on-disk state and generate a fresh identity.
 *  Only use when the server has lost the bundle and you need to restart
 *  with a new identity. Destroys all peer sessions. */
export const regenerateCrypto = (): Promise<void> => invoke('crypto_regenerate');

export const getIdentity = (): Promise<Identity> =>
  invoke('crypto_get_identity');

export const getBundleForUpload = (): Promise<BundleUpload> =>
  invoke('crypto_get_bundle_for_upload');

export const markKeysPublished = (): Promise<void> =>
  invoke('crypto_mark_keys_published');

export const establishSession = (
  peerUserId: string,
  peerIdentityKey: number[],
  peerSigningKey: number[],
  peerSignedPrekey: number[],
  peerSignedPrekeySignature: number[],
  peerOneTimePrekey: number[] | null,
): Promise<void> =>
  invoke('crypto_establish_session', {
    peerUserId,
    peerIdentityKey,
    peerSigningKey,
    peerSignedPrekey,
    peerSignedPrekeySignature,
    peerOneTimePrekey,
  });

export const hasSession = (peerUserId: string): Promise<boolean> =>
  invoke('crypto_has_session', { peerUserId });

export const encryptMessage = (
  peerUserId: string,
  plaintext: string,
): Promise<EncryptedMessage> =>
  invoke('crypto_encrypt', { peerUserId, plaintext });

export const decryptMessage = (
  peerUserId: string,
  peerIdentityKey: number[],
  messageType: number,
  ciphertext: number[],
): Promise<string> =>
  invoke('crypto_decrypt', {
    peerUserId,
    peerIdentityKey,
    messageType,
    ciphertext,
  });
