<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface DbConfig {
    host: string;
    port: number;
    database: string;
    username: string;
    password: string;
}

interface InvoiceRow {
    invoice_no: string;
    vendor_code: string;
    company_name: string;
    company_keyword: string;
    total_cost: number;
    receive_date: string;
    category: string;
}

interface PreviewData {
    invoices: InvoiceRow[];
    total_amount: number;
    row_count: number;
}

interface CarryForward {
    next_reg_no: string;
    next_running: number;
    next_po_no: number;
    next_purchase_no: number;
    remaining_balance: number;
}

interface GenerateResult {
    files: string[];
    total_rows: number;
    total_amount: number;
    carry_forward: CarryForward;
}

interface RoundHistoryEntry {
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

const props = defineProps<{
    dbConfig: DbConfig;
    dateFrom: string;
    dateTo: string;
    year: number;
    month: number;
    round: number;
    outputDir: string;
    previewData: PreviewData | null;
    budgetTotal: number;
    previousBalance: number;
    approvalDate: string;
    r2Carry: { next_reg_no: string; next_running: number; next_po_no: number } | null;
}>();

const emit = defineEmits<{
    (e: "update:budgetTotal", v: number): void;
    (e: "update:previousBalance", v: number): void;
    (e: "update:approvalDate", v: string): void;
    (e: "saveHistory", entry: RoundHistoryEntry): void;
}>();

const loading = ref(false);
const error = ref("");
const result = ref<GenerateResult | null>(null);

const THAI_MONTHS = [
    "มกราคม", "กุมภาพันธ์", "มีนาคม", "เมษายน", "พฤษภาคม", "มิถุนายน",
    "กรกฎาคม", "สิงหาคม", "กันยายน", "ตุลาคม", "พฤศจิกายน", "ธันวาคม",
];

const THAI_MONTHS_SHORT = [
    "ม.ค.", "ก.พ.", "มี.ค.", "เม.ย.", "พ.ค.", "มิ.ย.",
    "ก.ค.", "ส.ค.", "ก.ย.", "ต.ค.", "พ.ย.", "ธ.ค.",
];

// ── Thai date picker ──────────────────────────────────────────────────────
// Holds the native <input type="date"> value (YYYY-MM-DD).
// When changed, converts to Thai short format and emits to parent.
const approvalDatePicker = ref("");

function toThaiShortDate(htmlDate: string): string {
    if (!htmlDate) return "";
    const parts = htmlDate.split("-");
    if (parts.length !== 3) return "";
    const year = parseInt(parts[0], 10);
    const month = parseInt(parts[1], 10);
    const day = parseInt(parts[2], 10);
    if (isNaN(year) || isNaN(month) || isNaN(day)) return "";
    const thaiYear = (year + 543) % 100;
    return `${day} ${THAI_MONTHS_SHORT[month - 1] ?? ""} ${thaiYear}`;
}

function onApprovalDatePick() {
    emit("update:approvalDate", toThaiShortDate(approvalDatePicker.value));
}

const periodText = computed(() => {
    if (!props.year || !props.month) return "ยังไม่ได้เลือกช่วงวันที่";
    return `${THAI_MONTHS[props.month - 1]} ${props.year} รอบ ${props.round}`;
});

const canGenerate = computed(
    () =>
        props.previewData !== null &&
        props.previewData.row_count > 0 &&
        props.dateFrom !== "" &&
        props.dateTo !== "" &&
        props.budgetTotal > 0
);

// Preview of budget calculation for first invoice
const previewBalance = computed(() => {
    if (!props.previewData || props.previewData.row_count === 0) return null;
    const firstAmount = props.previewData.invoices[0]?.total_cost ?? 0;
    return props.previousBalance - firstAmount;
});

function formatMoney(n: number): string {
    return n.toLocaleString("th-TH", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
}

async function generate() {
    if (!canGenerate.value) return;
    loading.value = true;
    error.value = "";
    result.value = null;

    try {
        const res = await invoke<GenerateResult>("generate_cover_letters", {
            params: {
                db_config: { ...props.dbConfig },
                date_from: props.dateFrom,
                date_to: props.dateTo,
                year: props.year,
                month: props.month,
                round: props.round,
                budget_total: props.budgetTotal,
                previous_balance: props.previousBalance,
                approval_date: props.approvalDate.trim() || null,
                output_dir: props.outputDir,
            },
        });
        result.value = res;
    } catch (e) {
        error.value = String(e);
    } finally {
        loading.value = false;
    }
}

function saveToHistory() {
    if (!result.value) return;
    const now = new Date().toISOString();
    const monthShort = THAI_MONTHS_SHORT[props.month - 1] ?? "";
    // Use carry-forward from Report 2 for reg/po numbers (more accurate),
    // fall back to Report 3's carry-forward if Report 2 wasn't run yet.
    const regNo = props.r2Carry?.next_reg_no || result.value.carry_forward.next_reg_no || "";
    const running = props.r2Carry?.next_running ?? result.value.carry_forward.next_running ?? 0;
    const poNo = props.r2Carry?.next_po_no ?? result.value.carry_forward.next_po_no ?? 0;
    const entry: RoundHistoryEntry = {
        id: now,
        label: `${monthShort} ${props.year} รอบ ${props.round}`,
        fiscal_year: props.year,
        month: props.month,
        round: props.round,
        date_from: props.dateFrom,
        date_to: props.dateTo,
        next_reg_no: regNo,
        next_running: running,
        next_po_no: poNo,
        remaining_balance: result.value.carry_forward.remaining_balance,
        budget_total: props.budgetTotal,
        total_amount: result.value.total_amount,
        invoice_count: result.value.total_rows,
        created_at: now,
    };
    emit("saveHistory", entry);
}
</script>

<template>
<div class="report-wrap">
    <!-- ── Info banner ── -->
    <div class="card info-banner">
        <div class="banner-title">📄 เบิกยาปะหน้า (Disbursement Cover Letters)</div>
        <div class="banner-desc">
            สร้างบันทึกข้อความขออนุมัติเบิกเงิน — A4 Portrait PDF รวมทุกฉบับในไฟล์เดียว (1 หน้า/บิล)
        </div>
    </div>

    <!-- ── Data summary ── -->
    <div class="card">
        <div class="card-title">📊 ข้อมูลที่จะใช้สร้างรายงาน</div>

        <div v-if="!previewData" class="no-data">
            ⚠️ ยังไม่มีข้อมูล — กรุณาไปที่แท็บ 🔍 ดึงข้อมูล ก่อน
        </div>
        <div v-else>
            <div class="preview-summary">
                <div class="summary-stat">
                    <span class="summary-stat-label">ช่วงเวลา</span>
                    <span class="summary-stat-value period">{{ periodText }}</span>
                </div>
                <div class="summary-stat">
                    <span class="summary-stat-label">จำนวนบิล (หน้า)</span>
                    <span class="summary-stat-value">{{ previewData.row_count }} หน้า</span>
                </div>
                <div class="summary-stat">
                    <span class="summary-stat-label">ยอดเบิกจ่ายรอบนี้</span>
                    <span class="summary-stat-value money">{{ formatMoney(previewData.total_amount) }} บาท</span>
                </div>
            </div>
        </div>
    </div>

    <!-- ── Budget params ── -->
    <div class="card">
        <div class="card-title">💰 ตั้งค่างบประมาณ</div>
        <div class="card-desc">
            ยอดงบประมาณจะคำนวณแบบ running ต่อกันทุกบิล — โหลดค่าก่อนหน้าจากประวัติรอบได้
        </div>

        <div class="section-label">📋 ข้อมูลงวด</div>
        <div class="form-grid">
            <div class="form-group">
                <label>ปีงบประมาณ</label>
                <input type="text" :value="year > 0 ? String(year) : '—'" readonly />
            </div>
            <div class="form-group">
                <label>เดือน</label>
                <input type="text" :value="month > 0 ? THAI_MONTHS[month - 1] : '—'" readonly />
            </div>
            <div class="form-group">
                <label>รอบที่</label>
                <input type="text" :value="round" readonly />
            </div>
        </div>

        <div class="section-label" style="margin-top:16px">💰 งบประมาณ</div>
        <div class="form-grid">
            <div class="form-group">
                <label>ยอดเงินจัดสรรทั้งปี (บาท)</label>
                <input type="number" step="0.01" min="0" :value="budgetTotal"
                    @input="emit('update:budgetTotal', parseFloat(($event.target as HTMLInputElement).value) || 0)"
                    placeholder="5843812.60" />
                <span class="field-hint">งบประมาณที่ได้รับจัดสรรทั้งปีงบประมาณ</span>
            </div>
            <div class="form-group">
                <label>ยอดคงเหลือก่อนรอบนี้ (บาท)</label>
                <input type="number" step="0.01" min="0" :value="previousBalance"
                    @input="emit('update:previousBalance', parseFloat(($event.target as HTMLInputElement).value) || 0)"
                    placeholder="ยอดที่เหลือจากรอบที่แล้ว" />
                <span class="field-hint">ยอดคงเหลือสุดท้ายจากรอบก่อนหน้า (โหลดจากประวัติรอบได้)</span>
            </div>
            <div class="form-group">
                <label>วันที่ขออนุมัติ (แสดงบนเอกสาร)</label>
                <div class="date-picker-row">
                    <input type="date" v-model="approvalDatePicker" @change="onApprovalDatePick"
                        class="date-input-cal" />
                    <button v-if="approvalDate || approvalDatePicker" type="button" class="date-clear-btn"
                        @click="approvalDatePicker = ''; emit('update:approvalDate', '')" title="ล้างวันที่">✕</button>
                </div>
                <span v-if="approvalDate" class="field-hint date-thai-preview">
                    📅 {{ approvalDate }}
                </span>
                <span v-else class="field-hint">ปล่อยว่าง = ใช้วันที่รับของจากบิลแรก</span>
            </div>
        </div>

        <!-- Budget preview calculation -->
        <div v-if="previewData && previewData.row_count > 0" class="budget-preview">
            <div class="budget-preview-title">🔢 ตัวอย่างการคำนวณ (บิลแรก)</div>
            <div class="budget-row">
                <span class="budget-label">ยอดเงินจัดสรร</span>
                <span class="budget-val">{{ formatMoney(budgetTotal) }}</span>
            </div>
            <div class="budget-row">
                <span class="budget-label">ยอดคงเหลือก่อนรอบนี้</span>
                <span class="budget-val">{{ formatMoney(previousBalance) }}</span>
            </div>
            <div class="budget-row highlight">
                <span class="budget-label">เบิกจ่ายครั้งนี้ (บิลแรก)</span>
                <span class="budget-val debit">− {{ formatMoney(previewData.invoices[0]?.total_cost ?? 0) }}</span>
            </div>
            <div class="budget-row total">
                <span class="budget-label">ยอดคงเหลือหลังบิลแรก</span>
                <span class="budget-val" :class="(previewBalance ?? 0) < 0 ? 'negative' : 'positive'">
                    {{ formatMoney(previewBalance ?? 0) }}
                </span>
            </div>
        </div>

        <div class="info-box">
            💡 ระบบจะคำนวณยอดงบประมาณแบบ Running ต่อเนื่องทุกบิล:
            ยอดคงเหลือ[i] = ยอดคงเหลือ[i-1] − เบิกจ่ายครั้งนี้[i]
            ทุกค่าเป็นตัวเลขคงที่ใน PDF ไม่มีสูตร (ไม่เกิด #REF!)
        </div>

        <!-- Generate button -->
        <div class="actions">
            <button class="btn btn-primary btn-lg" :disabled="!canGenerate || loading" @click="generate">
                <span v-if="loading" class="spinner"></span>
                {{ loading ? "กำลังสร้าง PDF..." : "📄 สร้าง PDF เบิกยาปะหน้า" }}
            </button>
        </div>

        <div v-if="error" class="status-msg status-error" style="margin-top:12px">
            ❌ {{ error }}
        </div>
    </div>

    <!-- ── Result ── -->
    <div v-if="result" class="card">
        <div class="card-title">✅ สร้าง PDF สำเร็จ</div>

        <div class="result-card">
            <div class="result-card-title">📄 ไฟล์ที่สร้าง</div>
            <ul class="file-list">
                <li v-for="f in result.files" :key="f">
                    📄 <code>{{ fileName(f) }}</code>
                    <span class="file-path">{{ f }}</span>
                </li>
            </ul>
            <div class="result-stats">
                <span class="stat-chip">📦 {{ result.total_rows }} หน้า/บิล</span>
                <span class="stat-chip money">💰 {{ formatMoney(result.total_amount) }} บาท</span>
            </div>
        </div>

        <!-- Carry-forward info -->
        <div class="carry-box" style="margin-top:16px">
            <div class="carry-box-title">➡️ ค่าสำหรับรอบถัดไป (Carry-Forward)</div>
            <div class="carry-grid">
                <div class="carry-item">
                    <span class="carry-label">ยอดงบประมาณที่จัดสรร</span>
                    <span class="carry-val">{{ formatMoney(budgetTotal) }}</span>
                </div>
                <div class="carry-item">
                    <span class="carry-label">ยอดคงเหลือหลังรอบนี้</span>
                    <span class="carry-val">{{ formatMoney(result.carry_forward.remaining_balance) }}</span>
                </div>
            </div>
        </div>

        <div class="save-actions" style="margin-top:16px">
            <button class="btn btn-success" @click="saveToHistory">
                💾 บันทึกรอบนี้สู่ประวัติ
            </button>
        </div>
    </div>
</div>
</template>

<style scoped>
.date-picker-row {
    display: flex;
    align-items: center;
    gap: 6px;
}

.date-input-cal {
    flex: 1;
    padding: 8px 10px;
    border: 1.5px solid var(--clr-border, #d1d5db);
    border-radius: 8px;
    font-size: 14px;
    background: var(--clr-surface, #fff);
    color: var(--clr-text, #111827);
    cursor: pointer;
    transition: border-color 0.15s;
}

.date-input-cal:focus {
    outline: none;
    border-color: var(--clr-primary, #2563eb);
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.12);
}

.date-clear-btn {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--clr-text-muted, #6b7280);
    cursor: pointer;
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
}

.date-clear-btn:hover {
    background: var(--clr-danger-bg, #fee2e2);
    color: var(--clr-danger, #dc2626);
}

.date-thai-preview {
    color: var(--clr-primary, #2563eb);
    font-weight: 600;
}

.report-wrap {
    width: 100%;
}

/* ── Info banner ─────────────────────────────────────────────────────────────── */
.info-banner {
    background: linear-gradient(135deg, var(--c-primary-light) 0%, #FFD8C8 100%);
    border-color: #F0C4B8;
}

.banner-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--c-primary);
    margin-bottom: 5px;
}

.banner-desc {
    font-size: 14px;
    color: var(--c-primary-mid);
}

/* ── No-data placeholder ─────────────────────────────────────────────────────── */
.no-data {
    text-align: center;
    padding: 22px;
    color: var(--c-warn);
    font-size: 15px;
    background: var(--c-warn-bg);
    border-radius: var(--radius);
}

.period {
    color: var(--c-primary) !important;
}

.actions {
    margin-top: 22px;
}

/* ── Budget preview calculation ──────────────────────────────────────────────── */
/* #5C2C1E on #FFF0EC → ~11:1 ✓ */
.budget-preview {
    background: var(--c-primary-light);
    border: 1px solid #F0C4B8;
    border-radius: var(--radius);
    padding: 16px 18px;
    margin: 18px 0;
}

.budget-preview-title {
    font-size: 12px;
    font-weight: 700;
    color: var(--c-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 12px;
}

.budget-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 0;
    border-bottom: 1px solid #F0C4B8;
    font-size: 14px;
}

.budget-row:last-child {
    border-bottom: none;
}

/* warm amber highlight for debit row  */
.budget-row.highlight {
    background: rgba(251, 191, 36, 0.15);
    margin: 0 -6px;
    padding: 6px 6px;
    border-radius: 4px;
    border-bottom-color: transparent;
}

.budget-row.total {
    font-weight: 700;
    font-size: 15px;
    margin-top: 4px;
    padding-top: 10px;
    border-top: 2px solid var(--c-primary-mid);
    border-bottom: none;
}

.budget-label {
    color: var(--c-text-muted);
}

.budget-val {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--c-text);
}

/* #991B1B on #FFF0EC → ~5.6:1 ✓ */
.budget-val.debit {
    color: var(--c-error);
}

/* #166534 on #FFF0EC → ~7.2:1 ✓ */
.budget-val.positive {
    color: var(--c-success);
}

.budget-val.negative {
    color: var(--c-error);
}

/* ── Result section ──────────────────────────────────────────────────────────── */
.file-path {
    font-size: 11px;
    color: var(--c-text-muted);
    word-break: break-all;
    display: block;
    margin-top: 2px;
    padding-left: 20px;
}

.result-stats {
    display: flex;
    gap: 8px;
    margin-top: 14px;
    flex-wrap: wrap;
}

/* #166534 on #dcfce7 → 6.8:1 ✓ */
.stat-chip {
    background: #dcfce7;
    color: var(--c-success);
    border: 1px solid #86efac;
    border-radius: 999px;
    padding: 4px 14px;
    font-size: 13px;
    font-weight: 600;
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.stat-chip.money {
    background: var(--c-primary-light);
    color: var(--c-primary);
    border-color: #F0C4B8;
}

.save-actions {
    display: flex;
    gap: 10px;
}

/* ── Dark mode overrides ─────────────────────────────────────────────────────── */
@media (prefers-color-scheme: dark) {
    .info-banner {
        background: linear-gradient(135deg, #2A0808 0%, #350A0A 100%);
        border-color: #501515;
    }

    .banner-desc {
        color: var(--c-primary-mid);
    }

    .budget-preview {
        background: #2A0808;
        border-color: #501515;
    }

    .budget-row {
        border-bottom-color: #2e3820;
    }

    .budget-row.highlight {
        background: rgba(251, 191, 36, 0.10);
        border-bottom-color: transparent;
    }

    .budget-row.total {
        border-top-color: var(--c-primary-mid);
    }

    .budget-val.debit {
        color: var(--c-error);
    }

    .budget-val.positive {
        color: var(--c-success);
    }

    .stat-chip {
        background: #052e16;
        color: #4ade80;
        border-color: #166534;
    }

    .stat-chip.money {
        background: #2A0808;
        color: var(--c-primary);
        border-color: #501515;
    }
}
</style>
