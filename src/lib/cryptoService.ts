// src/lib/cryptoService.ts
//
// Glue between the Tauri crypto_* commands and your socket.io namespaces.
// Three things live here:
//
//   1. initCrypto(keysSocket)             — call once after socket connect.
//                                            Uploads the key bundle to /keys
//                                            on first launch, no-op after.
//   2. sendDM(channelSocket, keysSocket,  — encrypts plaintext, lazily
//          channelId, peerUserId, text)     establishes the session if
//                                            needed, emits channel_send.
//   3. decryptIncomingDM(keysSocket,      — call from your channel_message
//          myUserId, message)               handler. Returns null for own
//                                            echoes, plaintext otherwise.

import { writable, type Readable } from 'svelte/store';
import type { Socket } from 'socket.io-client';
import {
  attachCrypto,
  regenerateCrypto,
  getIdentity,
  getBundleForUpload,
  markKeysPublished,
  hasSession,
  establishSession,
  encryptMessage,
  decryptMessage,
  type Identity,
  type EncryptedMessage,
} from './crypto';

// ─── types matching the server's wire format ─────────────────────────

export interface Channel {
  channel_id: string;
  kind: 'dm' | 'text';
  name: string | null;
  dm_user_low: string | null;
  dm_user_low_username: string | null;
  dm_user_high: string | null;
  dm_user_high_username: string | null;
  created_at: string;
}

export interface ChannelMessage {
  message_id: string;
  channel_id: string;
  author_id: string;
  author_username: string;
  seq: number;
  /** For DMs this is `[message_type, ...ciphertext]`. For text channels
   *  it's the (already server-decrypted) plaintext bytes — we don't
   *  touch those here. */
  content: number[];
  created_at: string;
}

interface FetchedOneTimePrekey {
  prekey_id: number;
  public_key: number[];
}

interface FetchBundleOk {
  status: 'ok';
  user_id: string;
  identity_key: number[];
  signing_key: number[];
  signed_prekey: number[];
  signed_prekey_signature: number[];
  signed_prekey_id: number;
  one_time_prekey: FetchedOneTimePrekey | null;
}

type FetchBundleResponse = FetchBundleOk | { status: 'error'; reason: string };

type UploadBundleResponse =
  | { status: 'ok'; one_time_prekey_count: number }
  | { status: 'error'; reason: string };

type PrekeyCountResponse =
  | { status: 'ok'; count: number }
  | { status: 'error'; reason: string };

type ChannelSendResponse =
  | { status: 'ok'; message: ChannelMessage }
  | { status: 'error'; reason: string };

// ─── store ───────────────────────────────────────────────────────────

interface CryptoStatus {
  initialized: boolean;
  identity: Identity | null;
}

const _cryptoStatus = writable<CryptoStatus>({
  initialized: false,
  identity: null,
});

/** Read-only store: `{ initialized, identity }`. Subscribe in the UI if
 *  you want a visible "key sync in progress" state. */
export const cryptoStatus: Readable<CryptoStatus> = _cryptoStatus;

// ─── helpers ─────────────────────────────────────────────────────────

/** Given a DM channel and your own user id, return the other party's id.
 *  Returns null for non-DM channels or if you aren't a participant. */
export function peerOf(channel: Channel, myUserId: string): string | null {
  if (channel.kind !== 'dm') return null;
  if (channel.dm_user_low === myUserId) return channel.dm_user_high;
  if (channel.dm_user_high === myUserId) return channel.dm_user_low;
  return null;
}

/** Wire format for DM ciphertext: one byte for message type, then the
 *  Olm ciphertext bytes. Matches the server's `content: Vec<u8>` field. */
function encodeBlob(msg: EncryptedMessage): number[] {
  return [msg.message_type, ...msg.ciphertext];
}

function decodeBlob(content: number[]): {
  messageType: number;
  ciphertext: number[];
} {
  if (content.length < 1) throw new Error('empty content blob');
  return { messageType: content[0], ciphertext: content.slice(1) };
}

// ─── DM plaintext cache ──────────────────────────────────────────────
//
// Olm sessions advance their ratchet on every successful decrypt, which
// means a given ciphertext can only be decrypted ONCE. To survive history
// reloads (channel_sync) and own-message display (forward secrecy means
// we can't decrypt our own outgoing ciphertext), we cache plaintext for
// every DM keyed by (myUserId, messageId). Stored in localStorage so it
// survives app restarts.

function plaintextKey(userId: string, messageId: string): string {
  return `bonfire:plaintext:${userId}:${messageId}`;
}

export function cachePlaintext(
  userId: string,
  messageId: string,
  plaintext: string,
): void {
  try {
    localStorage.setItem(plaintextKey(userId, messageId), plaintext);
  } catch (e) {
    // Quota exceeded etc. — non-fatal, history falls back to placeholder
    // or re-attempts decrypt (which will fail).
    console.warn('cachePlaintext failed:', e);
  }
}

export function getPlaintext(userId: string, messageId: string): string | null {
  try {
    return localStorage.getItem(plaintextKey(userId, messageId));
  } catch {
    return null;
  }
}

// ─── init: bundle upload on first launch ─────────────────────────────

/**
 * Attach crypto state for this user and ensure the server has our bundle.
 *
 * Per-user namespacing means each user_id has its own on-disk state, so
 * switching between users (or logging out and back in as the same user)
 * preserves their identity and sessions.
 *
 * On first attach for a fresh user: generate bundle, upload, mark published.
 * On reattach with state already on disk: ask server (via prekey_count)
 * whether it has our bundle. If not — typical when the server DB has been
 * wiped — regenerate and re-upload. If yes, no-op.
 *
 * Call this once `myUserId` is known (from the auth ack) AND the /keys
 * socket has connected.
 */
export async function initCrypto(keysSocket: Socket, userId: string): Promise<void> {
  await attachCrypto(userId);
  let identity = await getIdentity();

  // Existence check: does the server actually have our bundle?
  let serverHasBundle = false;
  try {
    const countResp = (await keysSocket.emitWithAck(
      'prekey_count',
      {},
    )) as PrekeyCountResponse;
    serverHasBundle = countResp?.status === 'ok' && countResp.count > 0;
  } catch (e) {
    console.warn('prekey_count check failed; falling back to local state:', e);
    serverHasBundle = identity.keys_published;
  }

  if (identity.keys_published && serverHasBundle) {
    _cryptoStatus.set({ initialized: true, identity });
    return;
  }

  // Need to (re-)upload. If local says we already published, our account
  // has no unpublished OTKs to re-send, so regenerate fresh first. This
  // destroys peer sessions for this user, which is unavoidable when the
  // server has forgotten us.
  if (identity.keys_published) {
    console.warn(
      `server has no bundle for ${userId}; regenerating local identity`,
    );
    await regenerateCrypto();
    identity = await getIdentity();
  }

  const bundle = await getBundleForUpload();
  const resp = (await keysSocket.emitWithAck(
    'upload_bundle',
    bundle,
  )) as UploadBundleResponse;
  if (resp?.status !== 'ok') {
    throw new Error(`upload_bundle failed: ${resp?.reason ?? 'unknown'}`);
  }
  await markKeysPublished();

  _cryptoStatus.set({ initialized: true, identity });
}

// ─── send ────────────────────────────────────────────────────────────

/**
 * Encrypt `plaintext` for `peerUserId`, lazily fetching their bundle and
 * establishing a session if we don't have one yet, then emit
 * /channel::channel_send. Returns the server's `ChannelMessage`
 * (now carrying a real `seq`) so the caller can reconcile any optimistic
 * UI entry.
 */
export async function sendDM(
  channelSocket: Socket,
  keysSocket: Socket,
  channelId: string,
  peerUserId: string,
  plaintext: string,
): Promise<ChannelMessage> {
  if (!(await hasSession(peerUserId))) {
    await establishSessionFromBundle(keysSocket, peerUserId);
  }

  const enc = await encryptMessage(peerUserId, plaintext);
  const content = encodeBlob(enc);

  const resp = (await channelSocket.emitWithAck('channel_send', {
    channel_id: channelId,
    content,
  })) as ChannelSendResponse;

  if (resp?.status !== 'ok') {
    throw new Error(`channel_send failed: ${resp?.reason ?? 'unknown'}`);
  }
  return resp.message;
}

// ─── receive ─────────────────────────────────────────────────────────

/**
 * Decrypt a `channel_message` broadcast for a DM channel. Returns:
 *   - `null` if it's our own message echoed back (display the local
 *           optimistic plaintext instead).
 *   - the decrypted plaintext otherwise.
 *
 * For first-contact pre-key messages, lazily fetches the sender's bundle
 * to get their curve25519 identity key. This burns one of the sender's
 * OTKs as a side effect — acceptable for the deadline; an explicit
 * "fetch identity only" endpoint is a day-4+ optimization.
 *
 * Only call this for DM channels. For text channels the server already
 * returns plaintext.
 */
export async function decryptIncomingDM(
  keysSocket: Socket,
  myUserId: string,
  message: ChannelMessage,
): Promise<string | null> {
  if (message.author_id === myUserId) return null; // own echo

  const { messageType, ciphertext } = decodeBlob(message.content);

  let senderIdentityKey: number[] = [];
  if (!(await hasSession(message.author_id))) {
    if (messageType !== 0) {
      // Normal (post-handshake) message but we have no session. Either
      // we lost local state or sessions got out of sync. There's no
      // recovery from this in day-2 scope.
      throw new Error(
        `no session with ${message.author_id} and message is not a pre-key`,
      );
    }
    const bundle = await fetchBundle(keysSocket, message.author_id);
    senderIdentityKey = bundle.identity_key;
  }

  return await decryptMessage(
    message.author_id,
    senderIdentityKey,
    messageType,
    ciphertext,
  );
}

// ─── internals ───────────────────────────────────────────────────────

async function establishSessionFromBundle(
  keysSocket: Socket,
  peerUserId: string,
): Promise<void> {
  const bundle = await fetchBundle(keysSocket, peerUserId);
  await establishSession(
    peerUserId,
    bundle.identity_key,
    bundle.signing_key,
    bundle.signed_prekey,
    bundle.signed_prekey_signature,
    bundle.one_time_prekey?.public_key ?? null,
  );
}

async function fetchBundle(
  keysSocket: Socket,
  userId: string,
): Promise<FetchBundleOk> {
  const resp = (await keysSocket.emitWithAck('fetch_bundle', {
    user_id: userId,
  })) as FetchBundleResponse;
  if (resp?.status !== 'ok') {
    throw new Error(`fetch_bundle failed: ${resp?.reason ?? 'unknown'}`);
  }
  return resp;
}
