<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

type LoginSession = {
  sessionId: string;
  qrCodeDataUrl: string;
  message: string;
};

type LoginScanStatus =
  | { kind: 'waiting' }
  | { kind: 'scanned'; accountCount: number }
  | { kind: 'confirmed'; accountCount: number }
  | { kind: 'expired' }
  | { kind: 'error'; message: string };

type LoginAccount = {
  nickname: string;
  avatarUrl: string;
  expiresAt: string;
};

const sessionId = ref('');
const qrCodeDataUrl = ref('');
const statusMessage = ref('Official Account platform login only');
const errorMessage = ref('');
const isLoading = ref(false);
const isPolling = ref(false);
const account = ref<LoginAccount | null>(null);
const pollTimer = ref<number | null>(null);

const canFinalize = computed(() => statusMessage.value.includes('Confirmed'));

onBeforeUnmount(() => {
  stopPolling();
});

async function startLogin() {
  stopPolling();
  errorMessage.value = '';
  account.value = null;
  isLoading.value = true;
  statusMessage.value = 'Requesting Official Account QR code';

  try {
    const session = await invoke<LoginSession>('start_official_account_login');
    sessionId.value = session.sessionId;
    qrCodeDataUrl.value = session.qrCodeDataUrl;
    statusMessage.value = session.message;
    schedulePoll();
  } catch (error) {
    errorMessage.value = formatError(error);
    statusMessage.value = 'Official Account login is not connected';
  } finally {
    isLoading.value = false;
  }
}

async function pollLoginStatus() {
  if (!sessionId.value) {
    return;
  }

  isPolling.value = true;
  try {
    const status = await invoke<LoginScanStatus>('poll_official_account_login', {
      sessionId: sessionId.value,
    });

    switch (status.kind) {
      case 'waiting':
        statusMessage.value = 'Waiting for an Official Account operator to scan';
        schedulePoll();
        break;
      case 'scanned':
        statusMessage.value = status.accountCount > 0
          ? 'Scan received. Confirm login in WeChat'
          : 'No Official Account is available for this WeChat account';
        schedulePoll();
        break;
      case 'confirmed':
        statusMessage.value = 'Confirmed. Completing Official Account login';
        await finalizeLogin();
        break;
      case 'expired':
        statusMessage.value = 'QR code expired. Request a new one';
        qrCodeDataUrl.value = '';
        break;
      case 'error':
        statusMessage.value = 'Official Account login failed';
        errorMessage.value = status.message;
        break;
    }
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isPolling.value = false;
  }
}

async function finalizeLogin() {
  if (!sessionId.value) {
    return;
  }

  isLoading.value = true;
  errorMessage.value = '';

  try {
    account.value = await invoke<LoginAccount>('finalize_official_account_login', {
      sessionId: sessionId.value,
    });
    qrCodeDataUrl.value = '';
    sessionId.value = '';
    statusMessage.value = 'Official Account login is active';
  } catch (error) {
    errorMessage.value = formatError(error);
    statusMessage.value = 'Could not complete Official Account login';
  } finally {
    isLoading.value = false;
  }
}

async function logout() {
  stopPolling();
  errorMessage.value = '';
  isLoading.value = true;

  try {
    await invoke('logout_official_account_login');
    account.value = null;
    sessionId.value = '';
    qrCodeDataUrl.value = '';
    statusMessage.value = 'Official Account platform login only';
  } catch (error) {
    errorMessage.value = formatError(error);
  } finally {
    isLoading.value = false;
  }
}

function schedulePoll() {
  stopPolling();
  pollTimer.value = window.setTimeout(pollLoginStatus, 2000);
}

function stopPolling() {
  if (pollTimer.value !== null) {
    window.clearTimeout(pollTimer.value);
    pollTimer.value = null;
  }
}

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
</script>

<template>
  <section class="login-panel" aria-labelledby="official-account-login-title">
    <div class="login-panel__copy">
      <p class="section-label">Credential boundary</p>
      <h3 id="official-account-login-title">Official Account platform login</h3>
      <p>
        Use a WeChat Official Account operator account to unlock target account search,
        article synchronization, and local Markdown or HTML export.
      </p>
      <p class="login-panel__notice">
        Personal WeChat login is not supported. This desktop app is built for Official
        Account operators and researchers.
      </p>
    </div>

    <div class="login-panel__control" aria-live="polite">
      <div v-if="account" class="login-account">
        <img v-if="account.avatarUrl" :src="account.avatarUrl" alt="" />
        <div>
          <strong>{{ account.nickname }}</strong>
          <span>Session expires {{ account.expiresAt }}</span>
        </div>
      </div>

      <div v-else class="qr-stage">
        <img v-if="qrCodeDataUrl" :src="qrCodeDataUrl" alt="Official Account login QR code" />
        <div v-else class="qr-placeholder">
          <span>{{ isLoading ? 'Loading QR' : 'QR' }}</span>
        </div>
      </div>

      <div class="login-status">
        <strong>{{ statusMessage }}</strong>
        <span v-if="isPolling">Checking scan status...</span>
        <span v-else-if="canFinalize">Ready to complete login</span>
      </div>

      <p v-if="errorMessage" class="login-error">{{ errorMessage }}</p>

      <div class="login-actions">
        <button type="button" class="primary-button" :disabled="isLoading" @click="startLogin">
          {{ sessionId || qrCodeDataUrl ? 'Refresh QR code' : 'Start login' }}
        </button>
        <button type="button" class="secondary-button" :disabled="isLoading" @click="logout">
          Logout
        </button>
      </div>
    </div>
  </section>
</template>
