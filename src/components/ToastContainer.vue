<script setup lang="ts">
import { useToast } from "../composables/useToast";

const { toasts, remove, pause, resume } = useToast();

const icons: Record<string, string> = {
    success: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="10" fill="currentColor" opacity="0.15"/><path d="M6 10.5l2.5 2.5L14 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
    error: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="10" fill="currentColor" opacity="0.15"/><path d="M7 7l6 6M13 7l-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>`,
    warning: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="10" fill="currentColor" opacity="0.15"/><path d="M10 6v5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><circle cx="10" cy="14" r="1.2" fill="currentColor"/></svg>`,
    info: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="10" fill="currentColor" opacity="0.15"/><circle cx="10" cy="6.5" r="1.2" fill="currentColor"/><path d="M10 9v5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>`,
};
</script>

<template>
<Teleport to="body">
    <div class="toast-container" aria-live="polite">
        <TransitionGroup name="toast">
            <div
                v-for="toast in toasts"
                :key="toast.id"
                :class="['toast', `toast--${toast.type}`]"
                @mouseenter="pause(toast.id)"
                @mouseleave="resume(toast.id)"
                role="alert"
            >
                <div class="toast__icon" v-html="icons[toast.type]"></div>
                <div class="toast__body">
                    <div class="toast__title">{{ toast.title }}</div>
                    <div v-if="toast.message" class="toast__message">{{ toast.message }}</div>
                </div>
                <button class="toast__close" @click="remove(toast.id)" aria-label="ปิด">
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                        <path d="M3 3l8 8M11 3l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
                    </svg>
                </button>
                <div class="toast__progress">
                    <div
                        class="toast__progress-bar"
                        :style="{ width: toast.progress + '%' }"
                    ></div>
                </div>
            </div>
        </TransitionGroup>
    </div>
</Teleport>
</template>

<style>
/* Toast Container */
.toast-container {
    position: fixed;
    top: 16px;
    right: 16px;
    z-index: 99999;
    display: flex;
    flex-direction: column;
    gap: 10px;
    pointer-events: none;
    max-width: 420px;
    width: 100%;
}

/* Individual Toast */
.toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px 16px 16px;
    border-radius: 12px;
    background: #FFFCF9;
    border: 1px solid #EDD5C8;
    box-shadow:
        0 8px 30px rgba(140, 10, 20, 0.12),
        0 2px 8px rgba(140, 10, 20, 0.08);
    position: relative;
    overflow: hidden;
    backdrop-filter: blur(12px);
    cursor: default;
    min-width: 320px;
}

/* Type variants */
.toast--success {
    border-left: 4px solid #16A34A;
    color: #166534;
}

.toast--success .toast__progress-bar {
    background: linear-gradient(90deg, #16A34A, #4ade80);
}

.toast--error {
    border-left: 4px solid #DC2626;
    color: #991B1B;
}

.toast--error .toast__progress-bar {
    background: linear-gradient(90deg, #DC2626, #f87171);
}

.toast--warning {
    border-left: 4px solid #D97706;
    color: #92400E;
}

.toast--warning .toast__progress-bar {
    background: linear-gradient(90deg, #D97706, #fbbf24);
}

.toast--info {
    border-left: 4px solid #C8102E;
    color: #C8102E;
}

.toast--info .toast__progress-bar {
    background: linear-gradient(90deg, #C8102E, #E03050);
}

/* Toast children */
.toast__icon {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    margin-top: 1px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.toast__body {
    flex: 1;
    min-width: 0;
}

.toast__title {
    font-size: 14px;
    font-weight: 700;
    line-height: 1.35;
    letter-spacing: 0.01em;
    color: inherit;
}

.toast__message {
    font-size: 13px;
    line-height: 1.45;
    margin-top: 3px;
    opacity: 0.78;
    color: #5C2C1E;
    word-break: break-word;
}

.toast__close {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    border-radius: 6px;
    color: #9C6A58;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    margin: -2px -4px 0 0;
}

.toast__close:hover {
    background: rgba(200, 16, 46, 0.08);
    color: #C8102E;
}

/* Progress bar */
.toast__progress {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: rgba(0, 0, 0, 0.06);
}

.toast__progress-bar {
    height: 100%;
    border-radius: 0 3px 3px 0;
    transition: width 0.05s linear;
}

/* Transitions */
.toast-enter-active {
    animation: toast-in 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}

.toast-leave-active {
    animation: toast-out 0.28s cubic-bezier(0.55, 0, 1, 0.45) forwards;
}

.toast-move {
    transition: transform 0.3s cubic-bezier(0.22, 1, 0.36, 1);
}

@keyframes toast-in {
    0% {
        opacity: 0;
        transform: translateX(80px) scale(0.92);
    }
    100% {
        opacity: 1;
        transform: translateX(0) scale(1);
    }
}

@keyframes toast-out {
    0% {
        opacity: 1;
        transform: translateX(0) scale(1);
    }
    100% {
        opacity: 0;
        transform: translateX(80px) scale(0.92);
    }
}

/* Dark Mode */
@media (prefers-color-scheme: dark) {
    .toast {
        background: #220A08;
        border-color: #3D1515;
        box-shadow:
            0 8px 30px rgba(0, 0, 0, 0.50),
            0 2px 8px rgba(0, 0, 0, 0.35);
    }

    .toast--success {
        border-left-color: #4ade80;
        color: #4ade80;
    }

    .toast--error {
        border-left-color: #f87171;
        color: #f87171;
    }

    .toast--warning {
        border-left-color: #fbbf24;
        color: #fbbf24;
    }

    .toast--info {
        border-left-color: #FF6B80;
        color: #FF6B80;
    }

    .toast__message {
        color: #D4A090;
    }

    .toast__close {
        color: #8A6A60;
    }

    .toast__close:hover {
        background: rgba(255, 107, 128, 0.12);
        color: #FF6B80;
    }

    .toast__progress {
        background: rgba(255, 255, 255, 0.06);
    }
}
</style>
