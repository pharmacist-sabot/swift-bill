<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../composables/useToast";
import { FileText, BarChart3, AlertTriangle, Hash, Info, Eye, XCircle, Pencil, FileSpreadsheet, CheckCircle, ArrowRight, Save, Package, Banknote } from 'lucide-vue-next'

interface DbConfig {
    host: string;
    port: number;
    database: string;
    username: string;
    password: string;
}

interface CarryForward {
    next_reg_no: string;
    next_running: number;
    next_po_no: number;
    next_purchase_no: number;
    remaining_balance: number;
}

interface InvoiceSubmissionRow {
    seq: number;
    receive_date: string;
    invoice_no: string;
    reg_no: string;
    running_in_reg: number;
    invoice_date: string;
    company_name: string;
    category: string;
    total_amount: number;
}

interface InvoiceSubmissionPreview {
    rows: InvoiceSubmissionRow[];
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

interface PreviewData {
    invoices: unknown[];
    total_amount: number;
    row_count: number;
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
    startRegNo: string;
    startRunning: number;
}>();

const emit = defineEmits<{
    (e: "update:startRegNo", v: string): void;
    (e: "update:startRunning", v: number): void;
    (e: "saveHistory", entry: RoundHistoryEntry): void;
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

const editableRows = ref<InvoiceSubmissionRow[]>([]);
const carryForward = ref<CarryForward | null>(null);
const exportedFile = ref<string | null>(null);

// Computed properties

const periodText = computed(() => {
    if (!props.year || !props.month) return "ยังไม่ได้เลือกช่วงวันที่";
    return `${THAI_MONTHS[props.month - 1]} ${props.year} รอบ ${props.round}`;
});

const canPreview = computed(() =>
    props.previewData !== null &&
    props.previewData.row_count > 0 &&
    props.dateFrom !== "" &&
    props.dateTo !== "" &&
    props.startRegNo.trim() !== ""
);

const canExport = computed(() =>
    editableRows.value.length > 0 && !previewLoading.value
);

const exportedTotal = computed(() =>
    editableRows.value.reduce((s, r) => s + r.total_amount, 0)
);

// Helpers

function formatMoney(n: number): string {
    return n.toLocaleString("th-TH", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
}

// Actions

async function previewReport() {
    if (!canPreview.value) return;
    previewLoading.value = true;
    previewError.value = "";
    editableRows.value = [];
    exportedFile.value = null;
    exportError.value = "";
    carryForward.value = null;

    try {
        const preview = await invoke<InvoiceSubmissionPreview>("preview_invoice_submission", {
            params: {
                db_config: { ...props.dbConfig },
                date_from: props.dateFrom,
                date_to: props.dateTo,
                year: props.year,
                month: props.month,
                round: props.round,
                start_reg_no: props.startRegNo,
                start_running: props.startRunning,
                output_dir: props.outputDir,
            },
        });
        editableRows.value = preview.rows.map(r => ({ ...r }));
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
        const res = await invoke<GenerateResult>("export_invoice_submission_excel", {
            params: {
                rows: editableRows.value,
                year: props.year,
                month: props.month,
                round: props.round,
                start_reg_no: props.startRegNo,
                start_running: props.startRunning,
                output_dir: props.outputDir,
            },
        });
        exportedFile.value = res.files[0];
        carryForward.value = res.carry_forward;
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
        remaining_balance: carryForward.value.remaining_balance,
        budget_total: 0,
        total_amount: exportedTotal.value,
        invoice_count: editableRows.value.length,
        source_tab: "📋 ส่งหนี้เบิกยา",
        created_at: now,
    };
    emit("saveHistory", entry);
}
</script>

<template>
<div class="report-wrap">

    <!-- Info banner -->
    <div class="card info-banner">
        <div class="banner-title">
            <FileText :size="17" /> ส่งหนี้เบิกยา (Invoice Submission List)
        </div>
        <div class="banner-desc">สร้างรายการส่งหนี้สินและเอกสารเบิกเงิน</div>
    </div>

    <!-- Data summary from query -->
    <div class="card">
        <div class="card-title">
            <BarChart3 :size="17" /> ข้อมูลที่จะใช้สร้างรายงาน
        </div>
        <div v-if="!previewData" class="no-data">
            <AlertTriangle :size="14" /> ยังไม่มีข้อมูล — กรุณาไปที่แท็บ ดึงข้อมูล ก่อน
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
        <div class="card-title">
            <Hash :size="17" /> ตั้งค่าเลขทะเบียนคุม
        </div>
        <div class="card-desc">ค่าเหล่านี้ต่อเนื่องจากรอบก่อน — สามารถโหลดจากประวัติรอบได้</div>

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
                <span class="field-hint">กำหนดที่แท็บ ดึงข้อมูล</span>
            </div>
            <div class="form-group">
                <label>เลขทะเบียนคุมเริ่มต้น</label>
                <input type="text" :value="startRegNo"
                    @input="emit('update:startRegNo', ($event.target as HTMLInputElement).value)"
                    placeholder="เช่น 69ภ12" />
                <span class="field-hint">เลขที่ทะเบียนเล่มแรกของรอบนี้</span>
            </div>
            <div class="form-group">
                <label>ลำดับเริ่มต้นในสมุด (0–9)</label>
                <input type="number" min="0" max="9" :value="startRunning"
                    @input="emit('update:startRunning', parseInt(($event.target as HTMLInputElement).value) || 0)" />
                <span class="field-hint">ลำดับแรกในเล่มทะเบียน (เล่มใหม่ใส่ 0)</span>
            </div>
        </div>

        <div class="info-box" style="display:flex; align-items:flex-start; gap:8px;">
            <Info :size="14" style="flex-shrink:0" /> แต่ละสมุดทะเบียนมี 10 ลำดับ (0–9)
            เมื่อครบจะขึ้นเล่มใหม่โดยอัตโนมัติ
            เช่น 69ภ12 ลำดับ 8 → 69ภ12(8), 69ภ12(9), 69ภ13(0), …
        </div>

        <!-- Preview button -->
        <div class="actions">
            <button class="btn btn-primary btn-lg" :disabled="!canPreview || previewLoading" @click="previewReport">
                <span v-if="previewLoading" class="spinner"></span>
                <Eye v-if="!previewLoading" :size="16" />
                {{ previewLoading ? "กำลังโหลดตัวอย่าง..." : "แสดงตัวอย่าง" }}
            </button>
        </div>

        <div v-if="previewError" class="status-msg status-error" style="margin-top:12px">
            <XCircle :size="14" /> {{ previewError }}
        </div>
    </div>

    <!-- Editable preview table -->
    <div v-if="editableRows.length > 0" class="card">
        <div class="card-title">
            <Pencil :size="17" /> ตัวอย่างข้อมูล (แก้ไขได้)
        </div>
        <div class="card-desc">ตรวจสอบและแก้ไขข้อมูลก่อนส่งออก Excel</div>

        <div class="table-wrap">
            <table class="data-table edit-table">
                <thead>
                    <tr>
                        <th class="text-center">#</th>
                        <th>วันที่รับของ</th>
                        <th>เลขที่เอกสาร</th>
                        <th class="text-center">เลขทะเบียนคุม</th>
                        <th class="text-center">ลำดับ</th>
                        <th>วัน/เดือน/ปีใบส่งของ</th>
                        <th>รหัสบริษัท</th>
                        <th>ค่าใช้จ่ายเรื่อง</th>
                        <th class="text-right">จำนวนเงินรวม</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="row in editableRows" :key="row.seq">
                        <td class="text-center seq-cell">{{ row.seq }}</td>
                        <td><input v-model="row.receive_date" class="cell-input" /></td>
                        <td><input v-model="row.invoice_no" class="cell-input" /></td>
                        <td class="text-center reg-cell">{{ row.reg_no }}</td>
                        <td class="text-center reg-cell">{{ row.running_in_reg }}</td>
                        <td><input v-model="row.invoice_date" class="cell-input" /></td>
                        <td><input v-model="row.company_name" class="cell-input wide" /></td>
                        <td>
                            <select v-model="row.category" class="cell-select">
                                <option>ยา</option>
                                <option>วัสดุเภสัชกรรม</option>
                            </select>
                        </td>
                        <td class="text-right">
                            <input v-model.number="row.total_amount" type="number" step="0.01"
                                class="cell-input amount" />
                        </td>
                    </tr>
                </tbody>
                <tfoot>
                    <tr>
                        <td colspan="8" class="text-right">รวมทั้งสิ้น</td>
                        <td class="text-right total-cell">{{ formatMoney(exportedTotal) }}</td>
                    </tr>
                </tfoot>
            </table>
        </div>

        <!-- Export button -->
        <div class="actions">
            <button class="btn btn-success btn-lg" :disabled="!canExport || exportLoading" @click="exportExcel">
                <span v-if="exportLoading" class="spinner"></span>
                <FileSpreadsheet v-if="!exportLoading" :size="16" />
                {{ exportLoading ? "กำลังส่งออก Excel..." : "ส่งออก Excel" }}
            </button>
        </div>

        <div v-if="exportError" class="status-msg status-error" style="margin-top:12px">
            <XCircle :size="14" /> {{ exportError }}
        </div>
    </div>

    <!-- Export result -->
    <div v-if="exportedFile" class="card">
        <div class="card-title">
            <CheckCircle :size="17" /> ส่งออก Excel สำเร็จ
        </div>

        <div class="result-card">
            <div class="result-card-title">
                <FileSpreadsheet :size="15" /> ไฟล์ที่สร้าง
            </div>
            <ul class="file-list">
                <li>
                    <FileSpreadsheet :size="14" /> <code>{{ fileName(exportedFile) }}</code>
                    <span class="file-path">{{ exportedFile }}</span>
                </li>
            </ul>
            <div class="result-stats">
                <span class="stat-chip">
                    <Package :size="13" /> {{ editableRows.length }} รายการ
                </span>
                <span class="stat-chip money">
                    <Banknote :size="13" /> {{ formatMoney(exportedTotal) }} บาท
                </span>
            </div>
        </div>

        <!-- Carry-forward info -->
        <div v-if="carryForward" class="carry-box" style="margin-top:16px">
            <div class="carry-box-title">
                <ArrowRight :size="15" /> ค่าสำหรับรอบถัดไป (Carry-Forward)
            </div>
            <div class="carry-grid">
                <div class="carry-item">
                    <span class="carry-label">เลขทะเบียนคุมถัดไป</span>
                    <span class="carry-val">{{ carryForward.next_reg_no }}</span>
                </div>
                <div class="carry-item">
                    <span class="carry-label">ลำดับถัดไปในสมุด</span>
                    <span class="carry-val">{{ carryForward.next_running }}</span>
                </div>
            </div>
        </div>

        <div class="save-actions" style="margin-top:16px">
            <button class="btn btn-success" @click="saveToHistory">
                <Save :size="15" /> บันทึกรอบนี้สู่ประวัติ
            </button>
        </div>
    </div>

</div>
</template>

<style scoped>
.report-wrap {
    width: 100%;
}

/* Banner */
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

/* No data */
.no-data {
    text-align: center;
    padding: 20px;
    color: var(--c-warn);
    font-size: 15px;
    background: var(--c-warn-bg);
    border-radius: var(--radius);
}

.period {
    color: var(--c-primary) !important;
}

/* Actions */
.actions {
    margin-top: 22px;
}

/* Editable table */
.edit-table td {
    padding: 4px 6px;
    vertical-align: middle;
}

.seq-cell {
    color: var(--c-text-light);
    font-size: 13px;
    white-space: nowrap;
}

.reg-cell {
    color: var(--c-primary);
    font-weight: 600;
    white-space: nowrap;
    font-size: 13px;
}

.cell-input {
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 13px;
    width: 100%;
    min-width: 70px;
    background: transparent;
    color: var(--c-text);
    font-family: inherit;
}

.cell-input:focus {
    outline: none;
    border-color: var(--c-primary);
    background: var(--c-primary-light);
}

.cell-input.wide {
    min-width: 130px;
}

.cell-input.amount {
    text-align: right;
    min-width: 90px;
}

.cell-select {
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 13px;
    background: var(--c-bg-card);
    color: var(--c-text);
    font-family: inherit;
    cursor: pointer;
}

.cell-select:focus {
    outline: none;
    border-color: var(--c-primary);
}

.total-cell {
    font-weight: 700;
    color: var(--c-primary);
}

/* Result card */
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

.stat-chip {
    background: #dcfce7;
    color: var(--c-success);
    border: 1px solid #86efac;
    border-radius: 999px;
    padding: 4px 14px;
    font-size: 13px;
    font-weight: 600;
}

.stat-chip.money {
    background: var(--c-primary-light);
    color: var(--c-primary);
    border-color: #F0C4B8;
}

.save-actions {
    display: flex;
    gap: 10px;
}

/* Dark mode */
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
        background: #2A0808;
    }

    .cell-select {
        border-color: #444;
        background: var(--c-bg-card);
        color: var(--c-text);
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
