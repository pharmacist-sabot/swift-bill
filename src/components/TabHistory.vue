<script setup lang="ts">
import { ref } from "vue";
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
    source_tab?: string;
    created_at: string;
}

defineProps<{
    entries: RoundHistoryEntry[];
}>();

const emit = defineEmits<{
    (e: "loadEntry", entry: RoundHistoryEntry): void;
    (e: "deleteEntry", id: string): void;
}>();

const confirmingId = ref<string | null>(null);

const THAI_MONTHS = [
    "มกราคม", "กุมภาพันธ์", "มีนาคม", "เมษายน", "พฤษภาคม", "มิถุนายน",
    "กรกฎาคม", "สิงหาคม", "กันยายน", "ตุลาคม", "พฤศจิกายน", "ธันวาคม",
];

function formatMoney(n: number): string {
    return n.toLocaleString("th-TH", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function formatDate(yyyymmdd: string): string {
    if (!yyyymmdd || yyyymmdd.length < 8) return yyyymmdd;
    const y = parseInt(yyyymmdd.substring(0, 4)) + 543;
    const m = parseInt(yyyymmdd.substring(4, 6));
    const d = parseInt(yyyymmdd.substring(6, 8));
    return `${d} ${THAI_MONTHS[m - 1] ?? ""} ${y}`;
}

function formatCreatedAt(iso: string): string {
    try {
        const d = new Date(iso);
        return d.toLocaleString("th-TH", {
            year: "numeric",
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
        });
    } catch {
        return iso;
    }
}

function doDelete(id: string) {
    emit("deleteEntry", id);
    confirmingId.value = null;
}
</script>

<template>
<div class="history-wrap">
    <!-- Header card -->
    <div class="card header-card">
        <div class="card-title">📁 ประวัติรอบการทำงาน</div>
        <div class="card-desc">
            บันทึกข้อมูล carry-forward แต่ละรอบไว้ที่นี่ เพื่อดึงค่าไปใช้ในรอบถัดไปได้ทันที
            (บันทึกได้จากแต่ละหน้ารายงานหลังสร้าง PDF สำเร็จ)
        </div>
    </div>

    <!-- Empty state -->
    <div v-if="entries.length === 0" class="card empty-card">
        <div class="empty-icon">📂</div>
        <div class="empty-title">ยังไม่มีประวัติรอบ</div>
        <div class="empty-desc">
            หลังจากสร้าง PDF แต่ละรายงานสำเร็จ กด "💾 บันทึกรอบนี้สู่ประวัติ"
            เพื่อบันทึก carry-forward ไว้ใช้งานรอบถัดไป
        </div>
    </div>

    <!-- Entry list -->
    <div v-else class="entries">
        <div v-for="entry in entries" :key="entry.id" class="entry-card">
            <!-- Entry header -->
            <div class="entry-header">
                <div class="entry-title-row">
                    <span class="entry-label">{{ entry.label }}</span>
                    <span class="entry-round-badge">รอบ {{ entry.round }}</span>
                    <span v-if="entry.source_tab" class="source-tab-badge">{{ entry.source_tab }}</span>
                </div>
                <div class="entry-meta">
                    <span class="meta-chip">📅 {{ formatDate(entry.date_from) }} – {{ formatDate(entry.date_to)
                    }}</span>
                    <span class="meta-chip">📦 {{ entry.invoice_count }} บิล</span>
                    <span class="meta-chip money">💰 {{ formatMoney(entry.total_amount) }} บาท</span>
                    <span class="meta-chip muted">🕐 {{ formatCreatedAt(entry.created_at) }}</span>
                </div>
            </div>

            <!-- Carry-forward values -->
            <div class="carry-section">
                <div class="carry-title">➡️ ค่า Carry-Forward สำหรับรอบถัดไป (รอบ {{ entry.round + 1 }})</div>
                <div class="carry-values">
                    <div class="cv-item">
                        <span class="cv-label">เลขทะเบียนคุม</span>
                        <span class="cv-val reg">{{ entry.next_reg_no || "—" }}</span>
                    </div>
                    <div class="cv-item">
                        <span class="cv-label">ลำดับในสมุด</span>
                        <span class="cv-val">{{ entry.next_running }}</span>
                    </div>
                    <div class="cv-item">
                        <span class="cv-label">เลขขอซื้อ/PO</span>
                        <span class="cv-val">{{ entry.next_po_no || "—" }}</span>
                    </div>
                    <div class="cv-item">
                        <span class="cv-label">ยอดงบคงเหลือ</span>
                        <span class="cv-val money">{{ entry.remaining_balance > 0 ? formatMoney(entry.remaining_balance)
                            : "—" }}</span>
                    </div>
                    <div v-if="entry.budget_total > 0" class="cv-item">
                        <span class="cv-label">งบประมาณรวม</span>
                        <span class="cv-val">{{ formatMoney(entry.budget_total) }}</span>
                    </div>
                </div>
            </div>

            <!-- Actions -->
            <div class="entry-actions">
                <button class="btn btn-primary" @click="emit('loadEntry', entry)">
                    📥 โหลดค่านี้ไปใช้รอบถัดไป
                </button>
                <template v-if="confirmingId === entry.id">
                    <span class="confirm-text">ยืนยันลบรอบนี้?</span>
                    <button class="btn btn-danger" @click="doDelete(entry.id)">
                        ✓ ยืนยันลบ
                    </button>
                    <button class="btn btn-secondary" @click="confirmingId = null">
                        ✕ ยกเลิก
                    </button>
                </template>
                <button v-else class="btn btn-danger" @click="confirmingId = entry.id">
                    🗑️ ลบ
                </button>
            </div>
        </div>
    </div>

    <!-- How to use -->
    <div class="card tip-card">
        <div class="card-title">💡 วิธีใช้งานประวัติรอบ</div>
        <ol class="tip-list">
            <li>สร้าง PDF แต่ละรายงานสำเร็จแล้ว → กด <strong>💾 บันทึกรอบนี้สู่ประวัติ</strong></li>
            <li>เมื่อเริ่มรอบใหม่ → กด <strong>📥 โหลดค่านี้ไปใช้รอบถัดไป</strong></li>
            <li>ระบบจะ pre-fill ค่า carry-forward ให้ทุกแท็บอัตโนมัติ และสลับไปแท็บ 🔍 ดึงข้อมูล</li>
            <li>เลือกช่วงวันที่ใหม่ → ดึงข้อมูล → สร้างรายงานได้เลย</li>
        </ol>
    </div>
</div>
</template>

<style scoped>
.history-wrap {
    width: 100%;
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.header-card {
    background: linear-gradient(135deg, var(--c-primary-light) 0%, #FFD8C8 100%);
    border-color: #F0C4B8;
}

/* Empty state */
.empty-card {
    text-align: center;
    padding: 60px 20px !important;
}

.empty-icon {
    font-size: 52px;
    margin-bottom: 18px;
}

.empty-title {
    font-size: 17px;
    font-weight: 600;
    color: var(--c-text-muted);
    margin-bottom: 8px;
}

.empty-desc {
    font-size: 14px;
    color: var(--c-text-light);
    max-width: 420px;
    margin: 0 auto;
    line-height: 1.65;
}

/* Entry cards */
.entries {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.entry-card {
    background: var(--c-surface);
    border: 1px solid var(--c-border);
    border-radius: var(--radius-lg);
    padding: 18px 22px;
    box-shadow: var(--shadow);
    transition: border-color 0.15s, box-shadow 0.15s;
}

.entry-card:hover {
    border-color: var(--c-primary-mid);
    box-shadow: var(--shadow-md);
}

.entry-header {
    margin-bottom: 14px;
}

.entry-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
}

.entry-label {
    font-size: 17px;
    font-weight: 700;
    color: var(--c-text);
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.entry-round-badge {
    background: var(--c-primary-light);
    color: var(--c-primary);
    border: 1px solid #F0C4B8;
    border-radius: 999px;
    padding: 3px 12px;
    font-size: 13px;
    font-weight: 600;
}

.source-tab-badge {
    background: #EFF6FF;
    color: #1D4ED8;
    border: 1px solid #BFDBFE;
    border-radius: 999px;
    padding: 3px 12px;
    font-size: 12px;
    font-weight: 600;
}

.entry-meta {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
}

.meta-chip {
    font-size: 13px;
    color: var(--c-text-muted);
    background: var(--c-bg);
    border: 1px solid var(--c-border);
    border-radius: 999px;
    padding: 3px 11px;
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.meta-chip.money {
    color: var(--c-primary);
    background: var(--c-primary-light);
    border-color: #F0C4B8;
}

.meta-chip.muted {
    color: var(--c-text-light);
    font-size: 12px;
}

/* Carry values */
.carry-section {
    background: var(--c-primary-light);
    border: 1px solid #F0C4B8;
    border-radius: var(--radius);
    padding: 13px 16px;
    margin-bottom: 14px;
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.carry-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--c-primary);
    margin-bottom: 10px;
}

.carry-values {
    display: flex;
    gap: 24px;
    flex-wrap: wrap;
}

.cv-item {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 130px;
}

/* #5C2C1E on #FFF0EC → ~11:1 ✓ */
.cv-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--c-text-muted);
    font-weight: 600;
}

.cv-val {
    font-size: 16px;
    font-weight: 700;
    color: var(--c-text);
}

/* #C8102E on #FFF0EC → ~7.8:1 ✓ */
.cv-val.reg {
    color: var(--c-primary);
    font-family: "Consolas", "Fira Code", monospace;
}

.cv-val.money {
    color: var(--c-primary);
}

/* Entry actions */
.entry-actions {
    display: flex;
    gap: 10px;
    align-items: center;
}

.confirm-text {
    font-size: 13px;
    font-weight: 600;
    color: var(--c-error);
    padding: 0 6px;
    white-space: nowrap;
}

/* Tip card — warm cream tint */
.tip-card {
    background: var(--c-primary-light);
    border-color: #F0C4B8;
}

.tip-list {
    padding-left: 20px;
    line-height: 2.1;
    font-size: 14px;
    color: var(--c-text-muted);
}

.tip-list li {
    margin-bottom: 4px;
}

.tip-list strong {
    color: var(--c-text);
}

/* Dark Mode */
@media (prefers-color-scheme: dark) {
    .header-card {
        background: linear-gradient(135deg, #2A0808 0%, #350A0A 100%);
        border-color: #501515;
    }

    .entry-round-badge {
        background: #2A0808;
        border-color: #501515;
    }

    .source-tab-badge {
        background: #1E3A5F;
        color: #93C5FD;
        border-color: #1E40AF;
    }

    .meta-chip.money {
        background: #2A0808;
        color: var(--c-primary);
        border-color: #501515;
    }

    .carry-section {
        background: #2A0808;
        border-color: #501515;
    }

    .tip-card {
        background: #2A0808;
        border-color: #501515;
    }
}
</style>
