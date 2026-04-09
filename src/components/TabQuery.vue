<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useToast } from "../composables/useToast";
import { Search, CalendarDays, AlertTriangle, XCircle, BarChart3, Pill, Package, FolderOpen } from 'lucide-vue-next'

const toast = useToast();

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

const props = defineProps<{
    dbConfig: DbConfig;
    startDateHtml: string;
    endDateHtml: string;
    outputDir: string;
    previewData: PreviewData | null;
    round: number;
}>();

const emit = defineEmits<{
    (e: "update:startDateHtml", v: string): void;
    (e: "update:endDateHtml", v: string): void;
    (e: "update:outputDir", v: string): void;
    (e: "update:previewData", v: PreviewData | null): void;
    (e: "update:round", v: number): void;
}>();

const loading = ref(false);
const error = ref("");

const THAI_MONTHS = [
    "มกราคม", "กุมภาพันธ์", "มีนาคม", "เมษายน", "พฤษภาคม", "มิถุนายน",
    "กรกฎาคม", "สิงหาคม", "กันยายน", "ตุลาคม", "พฤศจิกายน", "ธันวาคม",
];

const THAI_MONTHS_SHORT = [
    "ม.ค.", "ก.พ.", "มี.ค.", "เม.ย.", "พ.ค.", "มิ.ย.",
    "ก.ค.", "ส.ค.", "ก.ย.", "ต.ค.", "พ.ย.", "ธ.ค.",
];

function toApiDate(html: string): string {
    return html.replace(/-/g, "");
}

function buddhistYear(html: string): number {
    return parseInt(html.substring(0, 4)) + 543;
}

function getMonth(html: string): number {
    return parseInt(html.substring(5, 7));
}

function getDay(html: string): number {
    return parseInt(html.substring(8, 10));
}

const periodLabel = computed(() => {
    if (!props.startDateHtml || !props.endDateHtml) return "กรุณาเลือกช่วงวันที่";
    const sy = buddhistYear(props.startDateHtml);
    const sm = getMonth(props.startDateHtml);
    const sd = getDay(props.startDateHtml);
    const ey = buddhistYear(props.endDateHtml);
    const em = getMonth(props.endDateHtml);
    const ed = getDay(props.endDateHtml);
    if (sm === em && sy === ey) {
        return `${sd}–${ed} ${THAI_MONTHS[sm - 1]} ${sy}`;
    }
    return `${sd} ${THAI_MONTHS_SHORT[sm - 1]} – ${ed} ${THAI_MONTHS_SHORT[em - 1]} ${ey}`;
});

const isDbReady = computed(
    () => props.dbConfig.host.trim() !== "" && props.dbConfig.username.trim() !== ""
);
const canFetch = computed(
    () => isDbReady.value && props.startDateHtml !== "" && props.endDateHtml !== ""
);

const drugCount = computed(() =>
    props.previewData?.invoices.filter((i) => i.category === "ยา").length ?? 0
);
const supplyCount = computed(() =>
    props.previewData?.invoices.filter((i) => i.category === "วัสดุเภสัชกรรม").length ?? 0
);

async function browseFolder() {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: "เลือกโฟลเดอร์สำหรับบันทึก PDF",
        });
        if (selected && typeof selected === "string") {
            emit("update:outputDir", selected);
        }
    } catch (e) {
        console.error("Browse folder error:", e);
    }
}

function formatMoney(n: number): string {
    return n.toLocaleString("th-TH", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

async function fetchData() {
    if (!canFetch.value) return;
    loading.value = true;
    error.value = "";
    emit("update:previewData", null);
    try {
        const data = await invoke<PreviewData>("fetch_preview", {
            config: { ...props.dbConfig },
            dateFrom: toApiDate(props.startDateHtml),
            dateTo: toApiDate(props.endDateHtml),
        });
        emit("update:previewData", data);
        if (data.row_count === 0) {
            toast.warning("ไม่พบข้อมูล", "ไม่พบรายการบิลในช่วงวันที่ที่เลือก");
        } else {
            toast.success(
                "ดึงข้อมูลสำเร็จ",
                `พบ ${data.row_count} รายการ ยอดรวม ${data.total_amount.toLocaleString("th-TH", { minimumFractionDigits: 2 })} บาท`
            );
        }
    } catch (e) {
        error.value = String(e);
        toast.error("ดึงข้อมูลล้มเหลว", String(e));
    } finally {
        loading.value = false;
    }
}
</script>

<template>
<div class="query-wrap">
    <!-- Date range card -->
    <div class="card">
        <div class="card-title">
            <Search :size="17" /> เลือกช่วงวันที่และดึงข้อมูล
        </div>
        <div class="card-desc">เลือกช่วงวันที่แล้วกด "ดึงข้อมูล" เพื่อโหลดรายการบิลจาก INVS</div>

        <div class="form-grid-2" style="margin-top:12px">
            <div class="form-group">
                <label>วันที่เริ่มต้น</label>
                <input type="date" :value="startDateHtml"
                    @input="emit('update:startDateHtml', ($event.target as HTMLInputElement).value)" />
            </div>
            <div class="form-group">
                <label>วันที่สิ้นสุด</label>
                <input type="date" :value="endDateHtml"
                    @input="emit('update:endDateHtml', ($event.target as HTMLInputElement).value)" />
            </div>
            <div class="form-group">
                <label>รอบที่ (สำหรับรายงานทั้ง 3 ระบบ)</label>
                <input type="number" min="1" max="99" :value="round"
                    @input="emit('update:round', parseInt(($event.target as HTMLInputElement).value) || 1)" />
                <span class="field-hint">รอบภายในช่วงเวลาเดียวกัน เช่น รอบ 1, 2, 3…</span>
            </div>
            <div class="form-group full">
                <label>โฟลเดอร์ที่ต้องการบันทึก</label>
                <div class="input-with-browse">
                    <input type="text" :value="outputDir"
                        @input="emit('update:outputDir', ($event.target as HTMLInputElement).value)"
                        placeholder="เช่น C:\Reports หรือ /Users/me/Documents (ปล่อยว่าง = โฟลเดอร์ปัจจุบัน)" />
                    <button class="btn btn-browse" @click="browseFolder" type="button" title="เลือกโฟลเดอร์">
                        <FolderOpen :size="15" /> เลือก
                    </button>
                </div>
                <span class="field-hint">ระบบจะสร้างโฟลเดอร์ย่อย output/ ภายในโฟลเดอร์ที่ระบุ</span>
            </div>
        </div>

        <div v-if="startDateHtml && endDateHtml" class="period-badge">
            <CalendarDays :size="14" /> {{ periodLabel }}
        </div>

        <div v-if="!isDbReady" class="status-msg status-warn" style="margin-top:12px">
            <AlertTriangle :size="14" /> กรุณาตั้งค่าการเชื่อมต่อฐานข้อมูลก่อน (แท็บ ฐานข้อมูล)
        </div>

        <div class="actions">
            <button class="btn btn-primary btn-lg" :disabled="!canFetch || loading" @click="fetchData">
                <span v-if="loading" class="spinner"></span>
                {{ loading ? "กำลังโหลดข้อมูล..." : "ดึงข้อมูล" }}
            </button>
        </div>

        <div v-if="error" class="status-msg status-error" style="margin-top:12px">
            <XCircle :size="14" /> {{ error }}
        </div>
    </div>

    <!-- Preview results -->
    <div v-if="previewData" class="card">
        <div class="card-title">
            <BarChart3 :size="17" /> ผลการดึงข้อมูล
        </div>

        <!-- Summary stats -->
        <div class="preview-summary">
            <div class="summary-stat">
                <span class="summary-stat-label">จำนวนรายการ</span>
                <span class="summary-stat-value">{{ previewData.row_count }}</span>
            </div>
            <div class="summary-stat">
                <span class="summary-stat-label">ยอดรวมทั้งหมด</span>
                <span class="summary-stat-value money">{{ formatMoney(previewData.total_amount) }}</span>
            </div>
            <div class="summary-stat">
                <span class="summary-stat-label">
                    <Pill :size="13" /> ยา
                </span>
                <span class="summary-stat-value">{{ drugCount }} ใบ</span>
            </div>
            <div class="summary-stat">
                <span class="summary-stat-label">
                    <Package :size="13" /> วัสดุเภสัชกรรม
                </span>
                <span class="summary-stat-value">{{ supplyCount }} ใบ</span>
            </div>
        </div>

        <div v-if="previewData.row_count === 0" class="empty-result">
            <AlertTriangle :size="14" /> ไม่พบข้อมูลในช่วงวันที่นี้ กรุณาเลือกช่วงวันที่ใหม่
        </div>

        <div v-else class="table-wrap">
            <table class="data-table">
                <thead>
                    <tr>
                        <th class="text-center">#</th>
                        <th>วันที่รับของ</th>
                        <th>เลขที่เอกสาร</th>
                        <th>รหัสบริษัท</th>
                        <th>ชื่อบริษัท</th>
                        <th class="text-center">ประเภท</th>
                        <th class="text-right">จำนวนเงิน (บาท)</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="(inv, idx) in previewData.invoices" :key="idx">
                        <td class="text-center num">{{ idx + 1 }}</td>
                        <td>{{ inv.receive_date }}</td>
                        <td><code>{{ inv.invoice_no }}</code></td>
                        <td><code>{{ inv.company_keyword }}</code></td>
                        <td>{{ inv.company_name }}</td>
                        <td class="text-center">
                            <span :class="['cat-badge', inv.category === 'ยา' ? 'cat-drug' : 'cat-supply']">
                                {{ inv.category }}
                            </span>
                        </td>
                        <td class="text-right money-cell">{{ formatMoney(inv.total_cost) }}</td>
                    </tr>
                </tbody>
                <tfoot>
                    <tr>
                        <td colspan="6" class="text-right">รวมทั้งสิ้น</td>
                        <td class="text-right">{{ formatMoney(previewData.total_amount) }}</td>
                    </tr>
                </tfoot>
            </table>
        </div>
    </div>

    <div v-else-if="!loading" class="card empty-card">
        <div class="empty-icon">
            <Search :size="44" stroke-width="1.5" />
        </div>
        <div class="empty-text">เลือกช่วงวันที่แล้วกด "ดึงข้อมูล" เพื่อดูรายการบิล</div>
        <div class="empty-hint">ข้อมูลที่ดึงมาจะถูกใช้สำหรับสร้างรายงานทั้ง 3 ระบบ</div>
    </div>
</div>
</template>

<style scoped>
.query-wrap {
    width: 100%;
}

.actions {
    margin-top: 16px;
}

.empty-result {
    text-align: center;
    padding: 20px;
    color: var(--c-warn);
    font-size: 15px;
}

.empty-card {
    text-align: center;
    padding: 56px 20px !important;
}

.empty-icon {
    color: var(--c-text-light);
    margin-bottom: 14px;
    display: flex;
    justify-content: center;
}

.empty-text {
    font-size: 16px;
    font-weight: 600;
    color: var(--c-text-muted);
    margin-bottom: 6px;
}

.empty-hint {
    font-size: 14px;
    color: var(--c-text-light);
}

.num {
    color: var(--c-text-light);
    font-size: 13px;
}

.money-cell {
    font-variant-numeric: tabular-nums;
    font-size: 14px;
}

code {
    font-family: "Consolas", "Fira Code", monospace;
    font-size: 12px;
    background: var(--c-primary-light);
    color: var(--c-primary);
    padding: 2px 6px;
    border-radius: 4px;
}

.input-with-browse {
    display: flex;
    gap: 8px;
    align-items: stretch;
}

.input-with-browse input {
    flex: 1;
    min-width: 0;
}

.btn-browse {
    white-space: nowrap;
    padding: 8px 16px;
    font-size: 14px;
    font-weight: 600;
    border: 1px solid var(--c-border);
    border-radius: var(--radius);
    background: var(--c-surface);
    color: var(--c-text);
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 4px;
}

.btn-browse:hover {
    background: var(--c-primary);
    color: #fff;
    border-color: var(--c-primary);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(99, 102, 241, 0.3);
}

.btn-browse:active {
    transform: translateY(0);
}
</style>
