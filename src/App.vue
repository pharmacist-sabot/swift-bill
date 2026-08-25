<script setup lang="ts">
import { ref, reactive, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import TabSettings from "./components/TabSettings.vue";
import TabQuery from "./components/TabQuery.vue";
import TabNumberLocks from "./components/TabNumberLocks.vue";
import TabReport1 from "./components/TabReport1.vue";
import TabReport2 from "./components/TabReport2.vue";
import TabReport3 from "./components/TabReport3.vue";
import TabHistory from "./components/TabHistory.vue";
import ToastContainer from "./components/ToastContainer.vue";
import { useToast } from "./composables/useToast";
import { Settings2, Database, Lock, FileText, ClipboardList, FileOutput, History } from 'lucide-vue-next'

const toast = useToast();

// Types

export interface DbConfig {
    host: string;
    port: number;
    database: string;
    username: string;
    password: string;
}

export interface InvoiceRow {
    invoice_no: string;
    vendor_code: string;
    company_name: string;
    company_keyword: string;
    total_cost: number;
    receive_date: string;
    category: string;
}

export interface PreviewData {
    invoices: InvoiceRow[];
    total_amount: number;
    row_count: number;
}

export interface CarryForward {
    next_reg_no: string;
    next_running: number;
    next_po_no: number;
    next_purchase_no: number;
    remaining_balance: number;
}

export interface GenerateResult {
    files: string[];
    total_rows: number;
    total_amount: number;
    carry_forward: CarryForward;
}

export interface SkippedLockedNumberSet {
    request_no: number;
    report_no: number;
    purchase_no: number;
    reason: string;
    note: string;
}

export interface ReceivingNumberingInfo {
    start_po_no: number;
    start_purchase_no: number;
    skipped_locked_sets: SkippedLockedNumberSet[];
}

export interface RoundHistoryEntry {
    id: string;
    label: string;
    fiscal_year: number;
    month: number;
    round: number;
    date_from: string;
    date_to: string;
    next_reg_no: string;
    next_running: number;
    next_po_no: number;
    next_purchase_no?: number;
    remaining_balance: number;
    budget_total: number;
    total_amount: number;
    invoice_count: number;
    created_at: string;
}

// Shared State

type TabId = "settings" | "query" | "numberLocks" | "report1" | "report2" | "report3" | "history";
const activeTab = ref<TabId>("settings");

const dbConfig = reactive<DbConfig>({
    host: "localhost",
    port: 1433,
    database: "INVS",
    username: "",
    password: "",
});

// Query / shared period state
const startDateHtml = ref(""); // YYYY-MM-DD (HTML date input format)
const endDateHtml = ref("");
const outputDir = ref("");
const previewData = ref<PreviewData | null>(null);

const dateFrom = computed(() => startDateHtml.value.replace(/-/g, ""));
const dateTo = computed(() => endDateHtml.value.replace(/-/g, ""));
const year = computed(() =>
    startDateHtml.value ? parseInt(startDateHtml.value.substring(0, 4)) + 543 : 0
);
const month = computed(() =>
    startDateHtml.value ? parseInt(startDateHtml.value.substring(5, 7)) : 0
);

// Shared round number (applies to all 3 reports)
const round = ref(1);

// DB connection status (set by TabSettings via connectionStatus event)
const dbConnected = ref<boolean | null>(null);

// Per-report unique fields
const r1Form = reactive({ startRegNo: "69ภ1", startRunning: 0 });
const r2Form = reactive({
    startPoNo: 1,
    startPurchaseNo: 1,
    startRegNo: "69ภ1",
    startRunning: 0,
    approvalDate: "",
});
const r3Form = reactive({
    budgetTotal: 5843812.6,
    previousBalance: 5843812.6,
    approvalDate: "",
});

// History
const historyEntries = ref<RoundHistoryEntry[]>([]);

// Carry-forward results stored from each report tab (for combined history save)
const r2Carry = ref<{
    next_reg_no: string;
    next_running: number;
    next_po_no: number;
    next_purchase_no: number;
} | null>(null);

// Lifecycle

onMounted(async () => {
    // Load the encrypted connection settings from the OS-keychain-backed store.
    const savedConfig = await invoke<DbConfig | null>("load_db_config").catch(() => null);
    if (savedConfig) {
        Object.assign(dbConfig, savedConfig);
    }
    try {
        historyEntries.value = await invoke<RoundHistoryEntry[]>("load_round_history");
    } catch (_) {
        /* ignore on fresh install */
    }
});

// History handlers

async function refreshHistory() {
    try {
        historyEntries.value = await invoke<RoundHistoryEntry[]>("load_round_history");
    } catch (_) { }
}

async function saveEntry(entry: RoundHistoryEntry) {
    try {
        await invoke("save_round_entry", { entry });
        await refreshHistory();
        toast.success("บันทึกประวัติสำเร็จ", `บันทึกรอบ ${entry.round} เรียบร้อยแล้ว`);
    } catch (e) {
        toast.error("บันทึกประวัติล้มเหลว", String(e));
    }
}

async function deleteEntry(id: string) {
    try {
        await invoke("delete_round_entry", { id });
        await refreshHistory();
        toast.success("ลบประวัติสำเร็จ", "ลบรายการประวัติเรียบร้อยแล้ว");
    } catch (e) {
        toast.error("ลบประวัติล้มเหลว", String(e));
    }
}

async function saveDbConfig() {
    try {
        await invoke("save_db_config", { config: { ...dbConfig } });
        toast.success("บันทึกการตั้งค่าสำเร็จ", "ข้อมูลการเชื่อมต่อถูกเข้ารหัสและบันทึกไว้ในเครื่องแล้ว");
    } catch (e) {
        toast.error("บันทึกการตั้งค่าล้มเหลว", String(e));
    }
}

function handleConnectionStatus(ok: boolean) {
    dbConnected.value = ok;
    if (ok) {
        toast.success("เชื่อมต่อสำเร็จ", "เชื่อมต่อฐานข้อมูล INVS ได้เรียบร้อย");
    } else {
        toast.error("เชื่อมต่อล้มเหลว", "ไม่สามารถเชื่อมต่อฐานข้อมูลได้ กรุณาตรวจสอบการตั้งค่า");
    }
}

function handleR2Carry(carry: { next_reg_no: string; next_running: number; next_po_no: number; next_purchase_no: number }) {
    r2Carry.value = carry;
}

async function applyHistoryEntry(entry: RoundHistoryEntry) {
    let numberingInfo: ReceivingNumberingInfo;
    try {
        numberingInfo = await invoke<ReceivingNumberingInfo>("normalize_receiving_start", {
            params: {
                fiscal_year: entry.fiscal_year,
                start_po_no: entry.next_po_no,
                start_purchase_no: entry.next_purchase_no ?? 1,
            },
        });
    } catch (e) {
        toast.error("โหลดประวัติล้มเหลว", `ไม่สามารถตรวจสอบเลขล็อกได้: ${String(e)}`);
        return;
    }

    // Pre-fill shared state
    round.value = entry.round + 1;

    // Pre-fill report 1 form
    r1Form.startRegNo = entry.next_reg_no;
    r1Form.startRunning = entry.next_running;

    // Pre-fill report 2 form
    r2Form.startPoNo = numberingInfo.start_po_no;
    r2Form.startPurchaseNo = numberingInfo.start_purchase_no;
    r2Form.startRegNo = entry.next_reg_no;
    r2Form.startRunning = entry.next_running;

    // Pre-fill report 3 form
    r3Form.budgetTotal = entry.budget_total;
    r3Form.previousBalance = entry.remaining_balance;

    // Switch to query tab so user can pick the new date range
    activeTab.value = "query";

    if (numberingInfo.skipped_locked_sets.length > 0) {
        toast.info(
            "โหลดประวัติสำเร็จ",
            `โหลดค่า carry-forward จากรอบ ${entry.round} แล้ว — ข้ามเลขล็อก ${numberingInfo.skipped_locked_sets.length} ชุดให้อัตโนมัติ`
        );
        return;
    }

    toast.info(
        "โหลดประวัติสำเร็จ",
        `โหลดค่า carry-forward จากรอบ ${entry.round} แล้ว — พร้อมทำงานรอบ ${entry.round + 1}`
    );
}

// Tabs metadata — icons used directly in sidebar template
</script>

<template>
<div class="app-root">

    <!-- ── Sidebar ─────────────────────────────────────────── -->
    <aside class="sidebar">

        <!-- Brand -->
        <div class="sidebar-brand">
            <div class="brand-icon" aria-hidden="true">
                <img src="/swift-bill-icon.svg" alt="" class="brand-icon-img" />
            </div>
            <div class="brand-text">
                <span class="brand-name">Swift Bill</span>
                <span class="brand-sub">โรงพยาบาลสระโบสถ์</span>
            </div>
        </div>

        <!-- Data context chip (shown once data is loaded) -->
        <div class="sidebar-context" v-if="previewData">
            <div class="context-chip">
                <span class="context-dot"></span>
                ข้อมูล {{ previewData.row_count }} รายการพร้อม
            </div>
        </div>

        <!-- Navigation -->
        <nav class="sidebar-nav">
            <span class="nav-section-label">ตั้งค่า</span>
            <button class="nav-item" :class="{ active: activeTab === 'settings' }"
                @click="activeTab = 'settings'">
                <Settings2 :size="15" :stroke-width="2" />
                <span class="nav-label">ฐานข้อมูล</span>
            </button>
            <button class="nav-item" :class="{ active: activeTab === 'query' }"
                @click="activeTab = 'query'">
                <Database :size="15" :stroke-width="2" />
                <span class="nav-label">ดึงข้อมูล</span>
            </button>
            <button class="nav-item" :class="{ active: activeTab === 'numberLocks' }"
                @click="activeTab = 'numberLocks'">
                <Lock :size="15" :stroke-width="2" />
                <span class="nav-label">ล็อกเลข</span>
            </button>

            <span class="nav-section-label">รายงาน</span>
            <button class="nav-item" :class="{ active: activeTab === 'report1' }"
                @click="activeTab = 'report1'">
                <FileText :size="15" :stroke-width="2" />
                <span class="nav-label">ส่งหนี้เบิกยา</span>
            </button>
            <button class="nav-item" :class="{ active: activeTab === 'report2' }"
                @click="activeTab = 'report2'">
                <ClipboardList :size="15" :stroke-width="2" />
                <span class="nav-label">สรุปรับยา</span>
            </button>
            <button class="nav-item" :class="{ active: activeTab === 'report3' }"
                @click="activeTab = 'report3'">
                <FileOutput :size="15" :stroke-width="2" />
                <span class="nav-label">เบิกยาปะหน้า</span>
            </button>

            <div class="nav-divider"></div>
            <button class="nav-item" :class="{ active: activeTab === 'history' }"
                @click="activeTab = 'history'">
                <History :size="15" :stroke-width="2" />
                <span class="nav-label">ประวัติรอบ</span>
            </button>
        </nav>

        <!-- Sidebar footer -->
        <div class="sidebar-footer">
            <div class="conn-indicator">
                <span class="conn-dot"
                    :class="dbConnected === true ? 'ok' : dbConnected === false ? 'fail' : 'unknown'">
                </span>
                <span class="conn-text">
                    {{ dbConnected === true ? 'INVS เชื่อมต่อแล้ว'
                     : dbConnected === false ? 'ยังไม่ได้เชื่อมต่อ'
                     : 'ยังไม่ได้ทดสอบ' }}
                </span>
            </div>
            <span class="sidebar-version">ภก.สุรเดช · v0.3.5</span>
        </div>

    </aside>

    <!-- ── Main content area ────────────────────────────────── -->
    <main class="main-area">
        <TabSettings v-show="activeTab === 'settings'" :db-config="dbConfig"
            @update:db-config="Object.assign(dbConfig, $event)" @save="saveDbConfig"
            @connection-status="handleConnectionStatus" />
        <TabQuery v-show="activeTab === 'query'" :db-config="dbConfig"
            v-model:start-date-html="startDateHtml"
            v-model:end-date-html="endDateHtml"
            v-model:output-dir="outputDir"
            v-model:preview-data="previewData"
            v-model:round="round" />
        <TabNumberLocks v-show="activeTab === 'numberLocks'" />
        <TabReport1 v-show="activeTab === 'report1'" :db-config="dbConfig"
            :date-from="dateFrom" :date-to="dateTo"
            :year="year" :month="month" :round="round"
            :output-dir="outputDir" :preview-data="previewData"
            v-model:start-reg-no="r1Form.startRegNo"
            v-model:start-running="r1Form.startRunning"
            @save-history="saveEntry" />
        <TabReport2 v-show="activeTab === 'report2'" :db-config="dbConfig"
            :date-from="dateFrom" :date-to="dateTo"
            :year="year" :month="month" :round="round"
            :output-dir="outputDir" :preview-data="previewData"
            v-model:start-po-no="r2Form.startPoNo"
            v-model:start-purchase-no="r2Form.startPurchaseNo"
            v-model:start-reg-no="r2Form.startRegNo"
            v-model:start-running="r2Form.startRunning"
            v-model:approval-date="r2Form.approvalDate"
            @save-history="saveEntry" @carry-result="handleR2Carry" />
        <TabReport3 v-show="activeTab === 'report3'" :db-config="dbConfig"
            :date-from="dateFrom" :date-to="dateTo"
            :year="year" :month="month" :round="round"
            :output-dir="outputDir" :preview-data="previewData"
            v-model:budget-total="r3Form.budgetTotal"
            v-model:previous-balance="r3Form.previousBalance"
            v-model:approval-date="r3Form.approvalDate"
            :r2-carry="r2Carry" @save-history="saveEntry" />
        <TabHistory v-show="activeTab === 'history'"
            :entries="historyEntries"
            @load-entry="applyHistoryEntry"
            @delete-entry="deleteEntry" />
    </main>

    <ToastContainer />
</div>
</template>

<style>
/* App shell only — design tokens and component styles in design-system.css */

/* ── Root: horizontal split ─────────────────────────────────── */
.app-root {
    display: flex;
    height: 100vh;
    overflow: hidden;
    background: var(--c-bg);
}

/* ══ SIDEBAR ══════════════════════════════════════════════════ */
.sidebar {
    width: 244px;
    flex-shrink: 0;
    background: var(--c-surface);
    color: var(--c-text);
    box-shadow: rgba(0, 0, 0, 0.08) 1px 0 0 0;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
}

/* ── Brand ─────────────────────────────────────────────────── */
.sidebar-brand {
    padding: 22px 18px 18px;
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
    border-bottom: 1px solid var(--c-border);
}

.brand-icon {
    width: 46px;
    height: 46px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

.brand-icon-img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
}

.brand-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
}

.brand-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--c-text);
    letter-spacing: -0.2px;
    line-height: 1.25;
}

.brand-sub {
    font-size: 11px;
    color: var(--c-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.45;
}

/* ── Context chip ───────────────────────────────────────────── */
.sidebar-context {
    padding: 14px 14px 6px;
    flex-shrink: 0;
}

.context-chip {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    background: var(--c-primary-light);
    box-shadow: rgba(200, 16, 46, 0.12) 0px 0px 0px 1px;
    border-radius: 9999px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--c-primary);
    line-height: 1.4;
}

.context-dot {
    width: 6px;
    height: 6px;
    background: var(--c-primary);
    border-radius: 50%;
    flex-shrink: 0;
    animation: pulse-dot 2.4s ease-in-out infinite;
}

@keyframes pulse-dot {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
}

/* ── Navigation ─────────────────────────────────────────────── */
.sidebar-nav {
    flex: 1;
    overflow-y: auto;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    scrollbar-width: none;
}
.sidebar-nav::-webkit-scrollbar { display: none; }

.nav-section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--c-text-light);
    padding: 14px 10px 6px;
    display: block;
    line-height: 1.4;
}

.nav-divider {
    height: 1px;
    background: var(--c-border);
    margin: 10px 2px;
}

.nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 12px 11px 14px;
    border-radius: 100px;
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    color: var(--c-text-muted);
    text-align: left;
    width: 100%;
    transition: background 0.12s, color 0.12s, box-shadow 0.12s;
    font-family: inherit;
    line-height: 1.45;
    flex-shrink: 0;
}

.nav-item svg {
    stroke: currentColor;
    flex-shrink: 0;
}

.nav-item:hover {
    background: var(--c-primary-light);
    color: var(--c-text);
}

.nav-item.active {
    background: #fff;
    color: var(--c-text);
    box-shadow: var(--shadow-ring);
    font-weight: 600;
}

.nav-label {
    flex: 1;
    letter-spacing: 0;
}

/* ── Sidebar footer ──────────────────────────────────────────── */
.sidebar-footer {
    padding: 16px 18px 18px;
    border-top: 1px solid var(--c-border);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.conn-indicator {
    display: flex;
    align-items: center;
    gap: 7px;
}

.conn-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
}

.conn-dot.ok {
    background: #16A34A;
    box-shadow: 0 0 0 3px rgba(22, 163, 74, 0.14);
}

.conn-dot.fail {
    background: var(--c-error);
    box-shadow: 0 0 0 3px rgba(185, 28, 28, 0.14);
}

.conn-dot.unknown {
    background: var(--c-border);
}

.conn-text {
    font-size: 12px;
    font-weight: 500;
    color: var(--c-text-muted);
    line-height: 1.4;
}

.sidebar-version {
    font-size: 10.5px;
    color: var(--c-text-light);
    letter-spacing: 0;
    line-height: 1.35;
}

/* ══ MAIN CONTENT AREA ════════════════════════════════════════ */
.main-area {
    flex: 1;
    overflow-y: auto;
    padding: 32px 40px 40px;
    background: var(--c-bg);
    min-width: 0;
}

.main-area > * {
    max-width: 1240px;
    margin: 0 auto;
}

/* ── Dark mode ───────────────────────────────────────────────── */
@media (prefers-color-scheme: dark) {
    .conn-dot.ok {
        background: #4ADE80;
        box-shadow: 0 0 0 3px rgba(74, 222, 128, 0.14);
    }
}
</style>
