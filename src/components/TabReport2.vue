<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../composables/useToast";

interface DbConfig {
    host: string;
    port: number;
    database: string;
    username: string;
    password: string;
}

interface PreviewData {
    invoices: unknown[];
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

interface ReceivingSummaryRow {
    approval_date: string;
    po_date: string;
    receive_date: string;
    company_code: string;
    total_amount: number;
    receiving_code: number;
    reg_no: string;
    running_in_reg: number;
    invoice_no: string;
    request_no: number;
    report_no: number;
    po_no: number;
}

interface ReceivingSummaryPreview {
    rows: ReceivingSummaryRow[];
    carry_forward: CarryForward;
    total_rows: number;
    total_amount: number;
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
    source_tab?: string;
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
    startPoNo: number;
    startPurchaseNo: number;
    startRegNo: string;
    startRunning: number;
    approvalDate: string;
}>();

const emit = defineEmits<{
    (e: "update:startPoNo", v: number): void;
    (e: "update:startPurchaseNo", v: number): void;
    (e: "update:startRegNo", v: string): void;
    (e: "update:startRunning", v: number): void;
    (e: "update:approvalDate", v: string): void;
    (e: "saveHistory", entry: RoundHistoryEntry): void;
    (e: "carryResult", carry: { next_reg_no: string; next_running: number; next_po_no: number; next_purchase_no: number }): void;
}>();

const toast = useToast();

const THAI_MONTHS = [
    "มกราคม", "กุมภาพันธ์", "มีนาคม", "เมษายน", "พฤษภาคม", "มิถุนายน",
    "กรกฎาคม", "สิงหาคม", "กันยายน", "ตุลาคม", "พฤศจิกายน", "ธันวาคม",
];
const THAI_MONTHS_SHORT = [
    "ม.ค.", "ก.พ.", "มี.ค.", "เม.ย.", "พ.ค.", "มิ.ย.",
    "ก.ค.", "ส.ค.", "ก.ย.", "ต.ค.", "พ.ย.", "ธ.ค.",
];

const previewLoading = ref(false);
const exportLoading = ref(false);
const previewError = ref("");
const exportError = ref("");
const editableRows = ref<ReceivingSummaryRow[]>([]);
const carryForward = ref<CarryForward | null>(null);
const exportedFile = ref<string | null>(null);

// Thai date picker for Approval Date
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

const canPreview = computed(
    () =>
        props.previewData !== null &&
        props.previewData.row_count > 0 &&
        props.dateFrom !== "" &&
        props.dateTo !== "" &&
        props.startRegNo.trim() !== ""
);

const canExport = computed(() => editableRows.value.length > 0 && !previewLoading.value);

const exportedTotal = computed(() =>
    editableRows.value.reduce((s, r) => s + r.total_amount, 0)
);

function formatMoney(n: number): string {
    return n.toLocaleString("th-TH", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
}

async function previewReport() {
    if (!canPreview.value) return;
    previewLoading.value = true;
    previewError.value = "";
    editableRows.value = [];
    exportedFile.value = null;
    exportError.value = "";
    carryForward.value = null;

    try {
        const preview = await invoke<ReceivingSummaryPreview>("preview_receiving_summary", {
            params: {
                db_config: { ...props.dbConfig },
                date_from: props.dateFrom,
                date_to: props.dateTo,
                year: props.year,
                month: props.month,
                round: props.round,
                start_po_no: props.startPoNo,
                start_purchase_no: props.startPurchaseNo,
                start_reg_no: props.startRegNo,
                start_running: props.startRunning,
                approval_date: props.approvalDate.trim() || null,
                output_dir: props.outputDir,
            },
        });
        editableRows.value = preview.rows.map((r) => ({ ...r }));
        carryForward.value = preview.carry_forward;
        toast.success("โหลดตัวอย่างสำเร็จ", `พบ ${preview.rows.length} รายการ`);
    } catch (e) {
        previewError.value = String(e);
        toast.error("โหลดตัวอย่างล้มเหลว", String(e));
    } finally {
        previewLoading.value = false;
    }
}

async function exportExcel() {
    if (!canExport.value) return;
    exportLoading.value = true;
    exportError.value = "";
    exportedFile.value = null;

    try {
        const res = await invoke<GenerateResult>("export_receiving_summary_excel", {
            params: {
                rows: editableRows.value,
                year: props.year,
                month: props.month,
                round: props.round,
                start_po_no: props.startPoNo,
                start_purchase_no: props.startPurchaseNo,
                start_reg_no: props.startRegNo,
                start_running: props.startRunning,
                output_dir: props.outputDir,
            },
        });
        exportedFile.value = res.files[0];
        carryForward.value = res.carry_forward;
        emit("carryResult", {
            next_reg_no: res.carry_forward.next_reg_no,
            next_running: res.carry_forward.next_running,
            next_po_no: res.carry_forward.next_po_no,
            next_purchase_no: res.carry_forward.next_purchase_no,
        });
        toast.success("ส่งออก Excel สำเร็จ", `บันทึกไฟล์เรียบร้อยแล้ว`);
    } catch (e) {
        exportError.value = String(e);
        toast.error("ส่งออก Excel ล้มเหลว", String(e));
    } finally {
        exportLoading.value = false;
    }
}

function saveToHistory() {
    if (!carryForward.value || !exportedFile.value) return;
    const now = new Date().toISOString();
    const monthShort = THAI_MONTHS_SHORT[props.month - 1] ?? "";
    const entry: RoundHistoryEntry = {
        id: now,
        label: `${monthShort} ${props.year} รอบ ${props.round}`,
        fiscal_year: props.year,
        month: props.month,
        round: props.round,
        date_from: props.dateFrom,
        date_to: props.dateTo,
        next_reg_no: carryForward.value.next_reg_no,
        next_running: carryForward.value.next_running,
        next_po_no: carryForward.value.next_po_no,
        next_purchase_no: carryForward.value.next_purchase_no,
        remaining_balance: carryForward.value.remaining_balance,
        budget_total: 0,
        total_amount: exportedTotal.value,
        invoice_count: editableRows.value.length,
        source_tab: "📊 สรุปรับยา",
        created_at: now,
    };
    emit("saveHistory", entry);
}
</script>

<template>
<div class="report-wrap">
    <!-- Info banner -->
    <div class="card info-banner">
        <div class="banner-title">📊 สรุปรับยา (Receiving Summary)</div>
        <div class="banner-desc">สร้างสรุปยอดรับยาประจำเดือน</div>
    </div>

    <!-- Data summary -->
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
                    <span class="summary-stat-label">จำนวนบิล</span>
                    <span class="summary-stat-value">{{ previewData.row_count }} รายการ</span>
                </div>
                <div class="summary-stat">
                    <span class="summary-stat-label">ยอดรวม</span>
                    <span class="summary-stat-value money">{{ formatMoney(previewData.total_amount) }} บาท</span>
                </div>
            </div>
        </div>
    </div>

    <!-- Report params -->
    <div class="card">
        <div class="card-title">🔢 ตั้งค่าเลขที่เอกสาร</div>
        <div class="card-desc">ค่าเหล่านี้ต่อเนื่องจากรอบก่อน — สามารถโหลดจากประวัติรอบได้</div>

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

        <div class="section-label" style="margin-top:16px">🔢 เลขที่เอกสาร (ต่อเนื่องจากรอบก่อน)</div>
        <div class="form-grid">
            <div class="form-group">
                <label>เลขขอซื้อ / รายงาน เริ่มต้น</label>
                <input type="number" min="1" :value="startPoNo"
                    @input="emit('update:startPoNo', parseInt(($event.target as HTMLInputElement).value) || 1)" />
                <span class="field-hint">ขอซื้อ (ลบ0033.302/) และ รายงาน/อนุมัติ</span>
            </div>
            <div class="form-group">
                <label>เลขใบสั่งซื้อ เริ่มต้น</label>
                <input type="number" min="1" :value="startPurchaseNo"
                    @input="emit('update:startPurchaseNo', parseInt(($event.target as HTMLInputElement).value) || 1)" />
                <span class="field-hint">ใบสั่งซื้อ…/{year} — นับอิสระจากเลขขอซื้อ</span>
            </div>
            <div class="form-group">
                <label>เลขทะเบียนคุมเริ่มต้น</label>
                <input type="text" :value="startRegNo"
                    @input="emit('update:startRegNo', ($event.target as HTMLInputElement).value)"
                    placeholder="เช่น 69ภ12" />
            </div>
            <div class="form-group">
                <label>ลำดับเริ่มต้นในสมุด (0–9)</label>
                <input type="number" min="0" max="9" :value="startRunning"
                    @input="emit('update:startRunning', parseInt(($event.target as HTMLInputElement).value) || 0)" />
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

        <!-- Preview button -->
        <div class="actions">
            <button class="btn btn-primary btn-lg" :disabled="!canPreview || previewLoading" @click="previewReport">
                <span v-if="previewLoading" class="spinner"></span>
                {{ previewLoading ? "กำลังโหลดตัวอย่าง..." : "👁️ แสดงตัวอย่าง" }}
            </button>
        </div>

        <div v-if="previewError" class="status-msg status-error" style="margin-top:12px">
            ❌ {{ previewError }}
        </div>
    </div>

    <!-- Editable preview table -->
    <div v-if="editableRows.length > 0" class="card">
        <div class="card-title">✏️ ตัวอย่างข้อมูล (แก้ไขได้)</div>
        <div class="card-desc">ตรวจสอบและแก้ไขข้อมูลก่อนส่งออก Excel — คอลัมน์สีเทาคำนวณอัตโนมัติ</div>

        <div class="table-wrap">
            <table class="data-table edit-table">
                <thead>
                    <tr>
                        <th>วันที่ขออนุมัติ</th>
                        <th>วันที่สั่งซื้อ</th>
                        <th>วันที่รับของ</th>
                        <th>รหัสบริษัท</th>
                        <th class="text-right">จำนวนเงินรวม</th>
                        <th class="text-center">รหัสลงรับยา</th>
                        <th class="text-center">เลขทะเบียนคุม</th>
                        <th class="text-center">ลำดับ</th>
                        <th>เลขที่ลงรับ</th>
                        <th class="text-center readonly-col">ขอซื้อ</th>
                        <th class="text-center readonly-col">รายงาน</th>
                        <th class="text-center readonly-col">ใบสั่งซื้อ</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="(row, idx) in editableRows" :key="idx">
                        <td><input v-model="row.approval_date" class="cell-input" /></td>
                        <td><input v-model="row.po_date" class="cell-input" /></td>
                        <td class="readonly-cell">{{ row.receive_date }}</td>
                        <td><input v-model="row.company_code" class="cell-input" /></td>
                        <td class="text-right">
                            <input v-model.number="row.total_amount" type="number" step="0.01"
                                class="cell-input amount" />
                        </td>
                        <td class="text-center">
                            <input v-model.number="row.receiving_code" type="number" class="cell-input num-center" />
                        </td>
                        <td class="text-center readonly-cell">{{ row.reg_no }}</td>
                        <td class="text-center readonly-cell">{{ row.running_in_reg }}</td>
                        <td class="readonly-cell">{{ row.invoice_no }}</td>
                        <td class="text-center readonly-cell">{{ row.request_no }}</td>
                        <td class="text-center readonly-cell">{{ row.report_no }}</td>
                        <td class="text-center readonly-cell">{{ row.po_no }}</td>
                    </tr>
                </tbody>
                <tfoot>
                    <tr>
                        <td colspan="4" class="text-right">รวมทั้งสิ้น</td>
                        <td class="text-right total-cell">{{ formatMoney(exportedTotal) }}</td>
                        <td colspan="7"></td>
                    </tr>
                </tfoot>
            </table>
        </div>

        <!-- Export button -->
        <div class="actions">
            <button class="btn btn-success btn-lg" :disabled="!canExport || exportLoading" @click="exportExcel">
                <span v-if="exportLoading" class="spinner"></span>
                {{ exportLoading ? "กำลังส่งออก Excel..." : "📊 ส่งออก Excel" }}
            </button>
        </div>

        <div v-if="exportError" class="status-msg status-error" style="margin-top:12px">
            ❌ {{ exportError }}
        </div>
    </div>

    <!-- Export result -->
    <div v-if="exportedFile" class="card">
        <div class="card-title">✅ ส่งออก Excel สำเร็จ</div>

        <div class="result-card">
            <div class="result-card-title">📊 ไฟล์ที่สร้าง</div>
            <ul class="file-list">
                <li>
                    📊 <code>{{ fileName(exportedFile) }}</code>
                    <span class="file-path">{{ exportedFile }}</span>
                </li>
            </ul>
            <div class="result-stats">
                <span class="stat-chip">📦 {{ editableRows.length }} รายการ</span>
                <span class="stat-chip money">💰 {{ formatMoney(exportedTotal) }} บาท</span>
            </div>
        </div>

        <!-- Carry-forward info -->
        <div v-if="carryForward" class="carry-box" style="margin-top:16px">
            <div class="carry-box-title">➡️ ค่าสำหรับรอบถัดไป (Carry-Forward)</div>
            <div class="carry-grid">
                <div class="carry-item">
                    <span class="carry-label">เลขทะเบียนคุมถัดไป</span>
                    <span class="carry-val">{{ carryForward.next_reg_no }}</span>
                </div>
                <div class="carry-item">
                    <span class="carry-label">ลำดับถัดไปในสมุด</span>
                    <span class="carry-val">{{ carryForward.next_running }}</span>
                </div>
                <div class="carry-item">
                    <span class="carry-label">เลขขอซื้อ/รายงาน ถัดไป</span>
                    <span class="carry-val">{{ carryForward.next_po_no }}</span>
                </div>
                <div class="carry-item">
                    <span class="carry-label">เลขใบสั่งซื้อ ถัดไป</span>
                    <span class="carry-val">{{ carryForward.next_purchase_no }}</span>
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

/* Editable table */

.edit-table {
    min-width: 1100px;
}

.edit-table td {
    padding: 4px 5px;
    vertical-align: middle;
}

.cell-input {
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 12px;
    width: 100%;
    min-width: 64px;
    background: transparent;
    color: var(--c-text);
    font-family: inherit;
}

.cell-input:focus {
    outline: none;
    border-color: var(--c-primary);
    background: var(--c-primary-light);
}

.cell-input.amount {
    text-align: right;
    min-width: 88px;
}

.cell-input.num-center {
    text-align: center;
    min-width: 52px;
}

.readonly-cell {
    color: var(--c-text-muted);
    font-size: 12px;
    background: var(--c-bg);
    white-space: nowrap;
}

.readonly-col {
    color: var(--c-text-muted);
    font-size: 11px;
}

.total-cell {
    font-weight: 700;
    color: var(--c-primary);
}

/* Result */

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

@media (prefers-color-scheme: dark) {
    .info-banner {
        background: linear-gradient(135deg, #2A0808 0%, #350A0A 100%);
        border-color: #501515;
    }

    .banner-desc {
        color: var(--c-primary-mid);
    }

    .cell-input {
        border-color: #444;
        color: var(--c-text);
    }

    .cell-input:focus {
        border-color: var(--c-primary);
        background: #2A0808;
    }

    .readonly-cell {
        background: #1a1a1a;
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
