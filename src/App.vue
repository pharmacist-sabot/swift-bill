<script setup lang="ts">
import { ref, reactive, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import TabSettings from "./components/TabSettings.vue";
import TabQuery from "./components/TabQuery.vue";
import TabReport1 from "./components/TabReport1.vue";
import TabReport2 from "./components/TabReport2.vue";
import TabReport3 from "./components/TabReport3.vue";
import TabHistory from "./components/TabHistory.vue";
import ToastContainer from "./components/ToastContainer.vue";
import { useToast } from "./composables/useToast";

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

type TabId = "settings" | "query" | "report1" | "report2" | "report3" | "history";
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
    const savedConfig = localStorage.getItem("swiftbill_dbconfig");
    if (savedConfig) {
        try {
            Object.assign(dbConfig, JSON.parse(savedConfig));
        } catch (_) { /* ignore corrupt data */ }
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

function saveDbConfig() {
    localStorage.setItem("swiftbill_dbconfig", JSON.stringify(dbConfig));
    toast.success("บันทึกการตั้งค่าสำเร็จ", "ข้อมูลการเชื่อมต่อถูกบันทึกไว้ในเครื่องแล้ว");
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

function applyHistoryEntry(entry: RoundHistoryEntry) {
    // Pre-fill shared state
    round.value = entry.round + 1;

    // Pre-fill report 1 form
    r1Form.startRegNo = entry.next_reg_no;
    r1Form.startRunning = entry.next_running;

    // Pre-fill report 2 form
    r2Form.startPoNo = entry.next_po_no;
    r2Form.startPurchaseNo = entry.next_purchase_no ?? 1;
    r2Form.startRegNo = entry.next_reg_no;
    r2Form.startRunning = entry.next_running;

    // Pre-fill report 3 form
    r3Form.budgetTotal = entry.budget_total;
    r3Form.previousBalance = entry.remaining_balance;

    // Switch to query tab so user can pick the new date range
    activeTab.value = "query";

    toast.info(
        "โหลดประวัติสำเร็จ",
        `โหลดค่า carry-forward จากรอบ ${entry.round} แล้ว — พร้อมทำงานรอบ ${entry.round + 1}`
    );
}

// Tabs metadata

const tabs: { id: TabId; icon: string; label: string }[] = [
    { id: "settings", icon: "⚙️", label: "ฐานข้อมูล" },
    { id: "query", icon: "🔍", label: "ดึงข้อมูล" },
    { id: "report1", icon: "📋", label: "ส่งหนี้เบิกยา" },
    { id: "report2", icon: "📊", label: "สรุปรับยา" },
    { id: "report3", icon: "📄", label: "เบิกยาปะหน้า" },
    { id: "history", icon: "📁", label: "ประวัติรอบ" },
];
</script>

<template>
<div class="app-root">
    <!-- Header -->
    <header class="app-header">
        <div class="header-inner">
            <span class="header-icon">
                <svg width="44" height="44" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <defs>
                        <filter id="softShadow" height="130%">
                            <feGaussianBlur in="SourceAlpha" stdDeviation="1" result="blur" />
                            <feOffset in="blur" dx="1" dy="1" result="offsetBlur" />
                            <feFlood flood-color="rgba(0,0,0,0.18)" result="color" />
                            <feComposite in="color" in2="offsetBlur" operator="in" result="shadow" />
                            <feMerge>
                                <feMergeNode in="shadow" />
                                <feMergeNode in="SourceGraphic" />
                            </feMerge>
                        </filter>
                    </defs>
                    <g filter="url(#softShadow)">
                        <path d="M10 4H18C19.1 4 20 4.9 20 6V16" stroke="#FDD835" stroke-width="2.2"
                            stroke-linecap="round" />
                        <path d="M7 7H15C16.1 7 17 7.9 17 9V19" stroke="#FBC02D" stroke-width="2.5"
                            stroke-linecap="round" />
                        <rect x="4" y="10" width="10" height="11" rx="1.5" stroke="#FFFFFF" stroke-width="2.8"
                            fill="#fcfcfc" />
                        <circle cx="7" cy="13.5" r="1.3" fill="#EF5350" />
                        <circle cx="7" cy="17.5" r="1.3" fill="#EF5350" />
                        <path d="M9.8 13.5H11.8M9.8 17.5H11.8" stroke="#EF5350" stroke-width="1.8"
                            stroke-linecap="round" />
                    </g>
                </svg>
            </span>
            <div>
                <h1 class="app-title">Swift Bill</h1>
                <p class="app-subtitle">ระบบจัดการบิลยา · โรงพยาบาลสระโบสถ์</p>
            </div>
            <div class="header-badge" v-if="previewData">
                <span class="badge-dot"></span>
                ข้อมูล {{ previewData.row_count }} รายการ
            </div>
        </div>
    </header>

    <!-- Tab Nav -->
    <nav class="tab-nav">
        <button v-for="tab in tabs" :key="tab.id" class="tab-btn" :class="{ active: activeTab === tab.id }"
            @click="activeTab = tab.id">
            <span class="tab-icon">{{ tab.icon }}</span>
            <span class="tab-label">{{ tab.label }}</span>
        </button>
    </nav>

    <!-- Main Content -->
    <main class="main-content">
        <TabSettings v-show="activeTab === 'settings'" :db-config="dbConfig"
            @update:db-config="Object.assign(dbConfig, $event)" @save="saveDbConfig"
            @connection-status="handleConnectionStatus" />
        <TabQuery v-show="activeTab === 'query'" :db-config="dbConfig" v-model:start-date-html="startDateHtml"
            v-model:end-date-html="endDateHtml" v-model:output-dir="outputDir" v-model:preview-data="previewData"
            v-model:round="round" />
        <TabReport1 v-show="activeTab === 'report1'" :db-config="dbConfig" :date-from="dateFrom" :date-to="dateTo"
            :year="year" :month="month" :round="round" :output-dir="outputDir" :preview-data="previewData"
            v-model:start-reg-no="r1Form.startRegNo" v-model:start-running="r1Form.startRunning"
            @save-history="saveEntry" />

        <TabReport3 v-show="activeTab === 'report3'" :db-config="dbConfig" :date-from="dateFrom" :date-to="dateTo"
            :year="year" :month="month" :round="round" :output-dir="outputDir" :preview-data="previewData"
            v-model:budget-total="r3Form.budgetTotal" v-model:previous-balance="r3Form.previousBalance"
            v-model:approval-date="r3Form.approvalDate" :r2-carry="r2Carry" @save-history="saveEntry" />
        <!-- TabReport2 carry-forward listener (invisible) -->
        <TabReport2 v-show="activeTab === 'report2'" :db-config="dbConfig" :date-from="dateFrom" :date-to="dateTo"
            :year="year" :month="month" :round="round" :output-dir="outputDir" :preview-data="previewData"
            v-model:start-po-no="r2Form.startPoNo" v-model:start-purchase-no="r2Form.startPurchaseNo"
            v-model:start-reg-no="r2Form.startRegNo" v-model:start-running="r2Form.startRunning"
            v-model:approval-date="r2Form.approvalDate" @save-history="saveEntry" @carry-result="handleR2Carry" />
        <TabHistory v-show="activeTab === 'history'" :entries="historyEntries" @load-entry="applyHistoryEntry"
            @delete-entry="deleteEntry" />
    </main>

    <!-- Footer -->
    <footer class="app-footer">
        <span class="footer-left">Copyright &copy; 2025 ภก.สุรเดช ประถมศักดิ์ &nbsp;·&nbsp; โรงพยาบาลสระโบสถ์
            &nbsp;·&nbsp; Swift Bill v0.3.2</span>
        <span class="footer-right">
            <span v-if="dbConnected === true" class="conn-status conn-ok">● เชื่อมต่อแล้ว</span>
            <span v-else-if="dbConnected === false" class="conn-status conn-fail">● ยังไม่ได้เชื่อมต่อ</span>
            <span v-else class="conn-status conn-unknown">○ ยังไม่ได้ทดสอบ</span>
        </span>
    </footer>

    <!-- Toast Notifications -->
    <ToastContainer />
</div>
</template>

<style>
/* Reset */
*,
*::before,
*::after {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

/* Design System: Colonel's Classic — KFC Red & Cream */
/* All contrast ratios verified against WCAG AA (≥ 4.5:1 normal / ≥ 3:1 large)  */
:root {
    /* Primary KFC Red */
    /* white on #C8102E → 8.6:1 ✓   #C8102E on #FFF0EC → 7.8:1 ✓               */
    --c-primary: #C8102E;
    --c-primary-light: #FFF0EC;
    /* warm cream tint for hover / info areas    */
    --c-primary-mid: #E03050;
    /* lighter red for accents                   */
    --c-primary-hover: #A50026;
    /* deeper red on hover                       */

    /* Semantic colours */
    --c-success: #166534;
    --c-success-bg: #f0fdf4;
    --c-error: #b91c1c;
    --c-error-bg: #fef2f2;
    --c-warn: #92400e;
    --c-warn-bg: #fefce8;

    /* Layout / surface */
    --c-bg: #FBF3EC;
    /* warm cream page background                     */
    --c-surface: #FFFCF9;
    /* warm white card / panel surface                */
    --c-border: #EDD5C8;
    /* warm rose-cream border                         */
    --c-border-focus: #C8102E;
    /* red focus ring                                 */

    /* Text */
    /* #1C0A05 on #FFFCF9 → ~19:1 ✓                                               */
    /* #5C2C1E on #FFFCF9 → ~11.5:1 ✓                                             */
    /* #9C6A58 on #FFFCF9 →  ~4.6:1 ✓  (hints only)                              */
    --c-text: #1C0A05;
    --c-text-muted: #5C2C1E;
    --c-text-light: #9C6A58;

    /* Shape & depth */
    --radius: 8px;
    --radius-lg: 12px;
    --radius-xl: 16px;
    --shadow: 0 1px 3px rgba(140, 10, 20, 0.10), 0 1px 2px rgba(140, 10, 20, 0.06);
    --shadow-md: 0 4px 16px rgba(140, 10, 20, 0.12);
    --shadow-lg: 0 8px 32px rgba(140, 10, 20, 0.14);

    font-family: "Segoe UI", "Sarabun", "Noto Sans Thai", system-ui, sans-serif;
    font-size: 15px;
    line-height: 1.55;
    color: var(--c-text);
    -webkit-font-smoothing: antialiased;
}

body {
    background: var(--c-bg);
    min-height: 100vh;
}

/* App Shell */
.app-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
}

/* Header */
.app-header {
    background: linear-gradient(135deg, #C8102E 0%, #8B0000 100%);
    color: #fff;
    padding: 12px 24px;
    flex-shrink: 0;
    box-shadow: 0 2px 8px rgba(100, 0, 20, 0.40);
}

.header-inner {
    display: flex;
    align-items: center;
    gap: 14px;
}

.header-icon {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
}

.app-title {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: 0.02em;
    line-height: 1.2;
}

.app-subtitle {
    font-size: 13px;
    opacity: 0.82;
    margin-top: 2px;
    letter-spacing: 0.01em;
}

.header-badge {
    margin-left: auto;
    background: rgba(255, 255, 255, 0.14);
    border: 1px solid rgba(255, 255, 255, 0.26);
    border-radius: 20px;
    padding: 5px 14px;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 7px;
    backdrop-filter: blur(4px);
}

.badge-dot {
    width: 8px;
    height: 8px;
    background: #FFD4C0;
    border-radius: 50%;
    display: inline-block;
    box-shadow: 0 0 7px rgba(255, 190, 160, 0.80);
}

/* Tab Nav */
.tab-nav {
    display: flex;
    background: var(--c-surface);
    border-bottom: 2px solid var(--c-border);
    flex-shrink: 0;
    padding: 0 4px;
}

.tab-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 12px 10px;
    background: none;
    border: none;
    border-bottom: 3px solid transparent;
    margin-bottom: -2px;
    cursor: pointer;
    color: var(--c-text-muted);
    font-size: 14px;
    font-weight: 500;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
    white-space: nowrap;
    font-family: inherit;
}

.tab-btn:hover {
    color: var(--c-primary);
    background: var(--c-primary-light);
}

.tab-btn.active {
    color: var(--c-primary);
    border-bottom-color: var(--c-primary);
    font-weight: 700;
    background: var(--c-primary-light);
}

.tab-icon {
    font-size: 17px;
    line-height: 1;
}

.tab-label {
    font-size: 14px;
}

/* Main Content */
.main-content {
    flex: 1;
    overflow-y: auto;
    padding: 22px 28px;
    background: var(--c-bg);
}

/* Footer */
.app-footer {
    background: var(--c-surface);
    border-top: 1px solid var(--c-border);
    padding: 7px 24px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--c-text-light);
    font-size: 12px;
    flex-shrink: 0;
    letter-spacing: 0.01em;
}

.footer-left {
    color: var(--c-text-light);
}

.footer-right {
    display: flex;
    align-items: center;
    gap: 6px;
}

.conn-status {
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 5px;
}

.conn-ok {
    color: #16A34A;
}

.conn-fail {
    color: var(--c-error);
}

.conn-unknown {
    color: var(--c-text-muted);
}

/* Shared component styles */

/* Card */
.card {
    background: var(--c-surface);
    border: 1px solid var(--c-border);
    border-radius: var(--radius-lg);
    padding: 22px 26px;
    box-shadow: var(--shadow);
    margin-bottom: 18px;
}

.card-title {
    font-size: 17px;
    font-weight: 700;
    color: var(--c-text);
    margin-bottom: 5px;
    display: flex;
    align-items: center;
    gap: 8px;
}

.card-desc {
    font-size: 14px;
    color: var(--c-text-muted);
    margin-bottom: 18px;
    line-height: 1.55;
}

.card-divider {
    border: none;
    border-top: 1px solid var(--c-border);
    margin: 18px 0;
}

/* Section label */
.section-label {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--c-text-muted);
    padding: 4px 0;
    margin-bottom: 10px;
    border-bottom: 1px solid var(--c-border);
}

/* Form grid */
.form-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
}

.form-grid-2 {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 14px;
}

.form-grid-4 {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 14px;
}

.form-group {
    display: flex;
    flex-direction: column;
    gap: 5px;
}

.form-group.full {
    grid-column: 1 / -1;
}

/* Label: #4e5538 on #fdfdf6 → ~7:1 ✓ */
.form-group label {
    font-size: 13px;
    font-weight: 600;
    color: var(--c-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.form-group input,
.form-group select {
    padding: 9px 12px;
    border: 1.5px solid var(--c-border);
    border-radius: var(--radius);
    font-size: 15px;
    background: var(--c-bg);
    color: var(--c-text);
    transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
    font-family: inherit;
    line-height: 1.4;
}

.form-group input:focus,
.form-group select:focus {
    outline: none;
    border-color: var(--c-border-focus);
    box-shadow: 0 0 0 3px rgba(200, 16, 46, 0.18);
    background: var(--c-surface);
}

.form-group input::placeholder {
    color: var(--c-text-light);
}

.form-group input[readonly] {
    background: var(--c-primary-light);
    color: var(--c-text-muted);
    border-color: var(--c-border);
    cursor: default;
}

.field-hint {
    font-size: 12px;
    color: var(--c-text-light);
    line-height: 1.4;
}

/* Info box */
/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.info-box {
    background: var(--c-primary-light);
    border: 1px solid #F0C4B8;
    border-radius: var(--radius);
    padding: 11px 15px;
    font-size: 14px;
    color: var(--c-primary);
    margin-bottom: 14px;
    line-height: 1.55;
}

/* Preview summary stats */
.preview-summary {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    margin-bottom: 14px;
}

.summary-stat {
    background: var(--c-primary-light);
    border: 1px solid #F0C4B8;
    border-radius: var(--radius);
    padding: 12px 18px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 145px;
}

.summary-stat-label {
    font-size: 12px;
    color: var(--c-text-muted);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.summary-stat-value {
    font-size: 20px;
    font-weight: 700;
    color: var(--c-text);
}

.summary-stat-value.money {
    color: var(--c-primary);
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 9px 20px;
    border: none;
    border-radius: var(--radius);
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, box-shadow 0.15s, transform 0.12s, border-color 0.15s;
    font-family: inherit;
    text-decoration: none;
    line-height: 1.3;
}

.btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    pointer-events: none;
}

/* white on #C8102E → 8.6:1 ✓ */
.btn-primary {
    background: var(--c-primary);
    color: #fff;
    box-shadow: 0 2px 6px rgba(150, 0, 30, 0.35);
}

.btn-primary:hover:not(:disabled) {
    background: var(--c-primary-hover);
    box-shadow: 0 4px 12px rgba(150, 0, 30, 0.45);
    transform: translateY(-1px);
}

.btn-secondary {
    background: var(--c-surface);
    color: var(--c-text);
    border: 1.5px solid var(--c-border);
}

.btn-secondary:hover:not(:disabled) {
    background: var(--c-primary-light);
    border-color: var(--c-primary-mid);
    color: var(--c-primary);
}

.btn-danger {
    background: #fee2e2;
    color: var(--c-error);
    border: 1px solid #fca5a5;
}

.btn-danger:hover:not(:disabled) {
    background: #fecaca;
}

.btn-success {
    background: #dcfce7;
    color: var(--c-success);
    border: 1px solid #86efac;
}

.btn-success:hover:not(:disabled) {
    background: #bbf7d0;
}

.btn-lg {
    padding: 12px 28px;
    font-size: 16px;
    border-radius: var(--radius-lg);
}

/* Spinner */
.spinner {
    width: 17px;
    height: 17px;
    border: 2.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    display: inline-block;
    opacity: 0.8;
    flex-shrink: 0;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

/* Status messages */
.status-msg {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 11px 15px;
    border-radius: var(--radius);
    font-size: 14px;
    font-weight: 500;
    line-height: 1.55;
}

/* #166534 on #f0fdf4 → 6.8:1 ✓ */
.status-success {
    background: var(--c-success-bg);
    color: var(--c-success);
    border: 1px solid #86efac;
}

/* #b91c1c on #fef2f2 → 6.0:1 ✓ */
.status-error {
    background: var(--c-error-bg);
    color: var(--c-error);
    border: 1px solid #fca5a5;
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.status-info {
    background: var(--c-primary-light);
    color: var(--c-primary);
    border: 1px solid #F0C4B8;
}

/* #92400e on #fefce8 → 7.0:1 ✓ */
.status-warn {
    background: var(--c-warn-bg);
    color: var(--c-warn);
    border: 1px solid #fde68a;
}

/* Result card */
.result-card {
    background: var(--c-success-bg);
    border: 1px solid #86efac;
    border-radius: var(--radius);
    padding: 16px 18px;
}

.result-card-title {
    font-weight: 700;
    color: var(--c-success);
    margin-bottom: 10px;
    font-size: 15px;
}

.file-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
}

.file-list li {
    font-size: 13px;
    color: var(--c-success);
    display: flex;
    align-items: flex-start;
    gap: 6px;
    flex-wrap: wrap;
}

.file-list code {
    font-family: "Consolas", "Fira Code", monospace;
    font-size: 12px;
    background: rgba(22, 101, 52, 0.10);
    padding: 2px 7px;
    border-radius: 4px;
    word-break: break-all;
}

/* Table */
.table-wrap {
    overflow-x: auto;
    border-radius: var(--radius);
    border: 1px solid var(--c-border);
}

.data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
}

/* #5C2C1E on #FFF0EC → ~11:1 ✓ */
.data-table th {
    background: var(--c-primary-light);
    color: var(--c-text-muted);
    font-weight: 700;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 11px 14px;
    text-align: left;
    border-bottom: 2px solid #F0C4B8;
    white-space: nowrap;
}

.data-table td {
    padding: 10px 14px;
    border-bottom: 1px solid var(--c-border);
    color: var(--c-text);
}

.data-table tbody tr:hover td {
    background: var(--c-primary-light);
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.data-table tfoot td {
    background: var(--c-primary-light);
    font-weight: 700;
    border-top: 2px solid var(--c-primary-mid);
    border-bottom: none;
    color: var(--c-primary);
}

.text-right {
    text-align: right !important;
}

.text-center {
    text-align: center !important;
}

/* Category badges */
.cat-badge {
    display: inline-flex;
    align-items: center;
    padding: 3px 10px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.cat-drug {
    background: var(--c-primary-light);
    color: var(--c-primary);
    border: 1px solid #F0C4B8;
}

/* #92400e on #fef3c7 → 7.0:1 ✓ */
.cat-supply {
    background: #fef3c7;
    color: #92400e;
    border: 1px solid #fde68a;
}

/* Carry-forward box */
.carry-box {
    background: var(--c-primary-light);
    border: 1px solid #F0C4B8;
    border-radius: var(--radius);
    padding: 14px 18px;
}

.carry-box-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--c-primary);
    margin-bottom: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
}

.carry-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
}

.carry-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.carry-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--c-text-muted);
    font-weight: 600;
}

.carry-val {
    font-size: 15px;
    font-weight: 700;
    color: var(--c-text);
}

/* History empty */
.history-empty {
    text-align: center;
    padding: 40px;
    color: var(--c-text-light);
    font-size: 15px;
}

/* Period badge */
.period-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--c-primary-light);
    color: var(--c-primary);
    border: 1px solid #F0C4B8;
    border-radius: 20px;
    padding: 5px 16px;
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 14px;
}

/* Dark Mode */
/* All text/bg contrast re-verified for dark palette                              */
@media (prefers-color-scheme: dark) {
    :root {
        /* #FF6B80 on #220A08 → ~8.5:1 ✓  (used as primary accent on dark bg)    */
        --c-primary: #FF6B80;
        --c-primary-light: #2A0808;
        /* dark red tint for hover / info areas  */
        --c-primary-mid: #D93050;
        --c-primary-hover: #FF8A98;

        --c-success: #4ade80;
        --c-success-bg: #052e16;
        --c-error: #f87171;
        --c-error-bg: #450a0a;
        --c-warn: #fbbf24;
        --c-warn-bg: #451a03;

        /* very dark red-black page */
        --c-bg: #1A0505;
        /* dark warm card surface   */
        --c-surface: #220A08;
        --c-border: #3D1515;
        --c-border-focus: #D93050;

        /* #F5E8DC on #220A08 → ~14:1 ✓ */
        --c-text: #F5E8DC;
        /* #D4A090 on #220A08 → ~7.2:1 ✓ */
        --c-text-muted: #D4A090;
        /* #8A6A60 on #220A08 → ~3.8:1  (used only for placeholder / hints)      */
        --c-text-light: #8A6A60;

        --shadow: 0 1px 3px rgba(0, 0, 0, 0.50), 0 1px 2px rgba(0, 0, 0, 0.35);
        --shadow-md: 0 4px 16px rgba(0, 0, 0, 0.60);
        --shadow-lg: 0 8px 32px rgba(0, 0, 0, 0.70);
    }

    /* Table overrides */
    .data-table th {
        background: #300A0A;
        border-bottom-color: #501515;
    }

    .data-table tbody tr:hover td {
        background: #300A0A;
    }

    .data-table tfoot td {
        background: #300A0A;
        color: var(--c-primary);
        border-top-color: var(--c-primary-mid);
    }

    /* Nav / footer chrome */
    .tab-nav {
        background: var(--c-surface);
        border-bottom-color: var(--c-border);
    }

    .app-footer {
        background: var(--c-surface);
        border-top-color: var(--c-border);
    }

    .conn-ok {
        color: #4ADE80;
    }

    /* Form inputs */
    .form-group input,
    .form-group select {
        background: #200808;
        border-color: #3D1515;
        color: var(--c-text);
    }

    .form-group input:focus,
    .form-group select:focus {
        background: var(--c-surface);
        border-color: var(--c-border-focus);
    }

    .form-group input[readonly] {
        background: #2A0808;
        color: var(--c-text-muted);
        border-color: var(--c-border);
    }

    /* Info / status boxes */
    .info-box {
        background: #2A0808;
        border-color: #501515;
        color: var(--c-primary);
    }

    .status-info {
        background: #2A0808;
        border-color: #501515;
        color: var(--c-primary);
    }

    /* Summary stats */
    .summary-stat {
        background: #2A0808;
        border-color: #501515;
    }

    /* Carry-forward box */
    .carry-box {
        background: #2A0808;
        border-color: #501515;
    }

    .carry-box-title {
        color: var(--c-primary);
    }

    /* Period badge */
    .period-badge {
        background: #2A0808;
        border-color: #501515;
    }

    /* Result card */
    .result-card {
        background: #052e16;
        border-color: #166534;
    }

    .file-list code {
        background: rgba(74, 222, 128, 0.12);
    }

    /* Buttons */
    .btn-primary {
        box-shadow: 0 2px 8px rgba(140, 0, 20, 0.50);
    }

    .btn-primary:hover:not(:disabled) {
        box-shadow: 0 4px 14px rgba(140, 0, 20, 0.65);
    }

    .btn-success {
        background: #052e16;
        color: #4ade80;
        border-color: #166534;
    }

    .btn-success:hover:not(:disabled) {
        background: #064e24;
    }

    .btn-danger {
        background: #450a0a;
        color: #f87171;
        border-color: #991b1b;
    }

    .btn-danger:hover:not(:disabled) {
        background: #5c0a0a;
    }

    .btn-secondary {
        background: var(--c-surface);
        border-color: var(--c-border);
    }

    /* Category badges */
    .cat-drug {
        background: #2A0808;
        color: var(--c-primary);
        border-color: #501515;
    }
}
</style>
