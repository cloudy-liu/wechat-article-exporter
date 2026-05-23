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
const statusMessage = ref('仅支持公众号平台登录');
const errorMessage = ref('');
const isLoading = ref(false);
const isPolling = ref(false);
const account = ref<LoginAccount | null>(null);
const pollTimer = ref<number | null>(null);
const loginPhase = ref<'idle' | 'waiting' | 'scanned' | 'confirmed' | 'active' | 'expired' | 'error'>('idle');

const canFinalize = computed(() => loginPhase.value === 'confirmed');

onBeforeUnmount(() => {
  stopPolling();
});

async function startLogin() {
  stopPolling();
  errorMessage.value = '';
  account.value = null;
  isLoading.value = true;
  loginPhase.value = 'waiting';
  statusMessage.value = '正在获取公众号平台登录二维码';

  try {
    const session = await invoke<LoginSession>('start_official_account_login');
    sessionId.value = session.sessionId;
    qrCodeDataUrl.value = session.qrCodeDataUrl;
    statusMessage.value = session.message || '请使用公众号平台运营者微信扫码登录';
    schedulePoll();
  } catch (error) {
    errorMessage.value = formatError(error);
    loginPhase.value = 'error';
    statusMessage.value = '公众号平台登录尚未连接';
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
        loginPhase.value = 'waiting';
        statusMessage.value = '等待公众号运营者扫码';
        schedulePoll();
        break;
      case 'scanned':
        loginPhase.value = 'scanned';
        statusMessage.value = status.accountCount > 0
          ? '已扫码，请在微信里确认登录'
          : '当前微信号没有可用的公众号权限';
        schedulePoll();
        break;
      case 'confirmed':
        loginPhase.value = 'confirmed';
        statusMessage.value = '已确认，正在完成公众号平台登录';
        await finalizeLogin();
        break;
      case 'expired':
        loginPhase.value = 'expired';
        statusMessage.value = '二维码已过期，请重新获取';
        qrCodeDataUrl.value = '';
        break;
      case 'error':
        loginPhase.value = 'error';
        statusMessage.value = '公众号平台登录失败';
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
    loginPhase.value = 'active';
    statusMessage.value = '公众号平台登录已生效';
  } catch (error) {
    errorMessage.value = formatError(error);
    loginPhase.value = 'error';
    statusMessage.value = '无法完成公众号平台登录';
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
    loginPhase.value = 'idle';
    statusMessage.value = '仅支持公众号平台登录';
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
      <p class="section-label">登录边界</p>
      <h3 id="official-account-login-title">公众号平台登录</h3>
      <p>
        使用公众号平台运营者账号登录后，可以搜索目标公众号、同步文章列表，并在本地导出 Markdown 或 HTML。
      </p>
      <p class="login-panel__notice">
        不支持个人微信号登录。当前桌面端面向公众号作者、运营者和内容研究场景。
      </p>
    </div>

    <div class="login-panel__control" aria-live="polite">
      <div v-if="account" class="login-account">
        <img v-if="account.avatarUrl" :src="account.avatarUrl" alt="" />
        <div>
          <strong>{{ account.nickname }}</strong>
          <span>登录有效期至 {{ account.expiresAt }}</span>
        </div>
      </div>

      <div v-else class="qr-stage">
        <img v-if="qrCodeDataUrl" :src="qrCodeDataUrl" alt="公众号平台登录二维码" />
        <div v-else class="qr-placeholder">
          <span>{{ isLoading ? '加载二维码' : '二维码' }}</span>
        </div>
      </div>

      <div class="login-status">
        <strong>{{ statusMessage }}</strong>
        <span v-if="isPolling">正在检查扫码状态...</span>
        <span v-else-if="canFinalize">可以完成登录</span>
      </div>

      <p v-if="errorMessage" class="login-error">{{ errorMessage }}</p>

      <div class="login-actions">
        <button type="button" class="primary-button" :disabled="isLoading" @click="startLogin">
          {{ sessionId || qrCodeDataUrl ? '刷新二维码' : '开始登录' }}
        </button>
        <button type="button" class="secondary-button" :disabled="isLoading" @click="logout">
          退出登录
        </button>
      </div>
    </div>
  </section>
</template>
