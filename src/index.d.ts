export interface SequenceStep {
  id: string
  afterHours?: number
  subject?: string | null
  template: string
}

export interface Sequence {
  id: string
  name?: string | null
  steps: SequenceStep[]
}

export interface Contact {
  id: string
  email?: string | null
  variables?: Record<string, unknown>
}

export interface EnrollmentEvent {
  type: string
  at: string | Date
  stepId?: string | null
  messageId?: string | null
  reason?: string | null
}

export interface Enrollment {
  id: string
  sequenceId: string
  contact: Contact
  enrolledAt: string | Date
  status?: 'active' | 'paused' | 'completed' | 'cancelled'
  events?: EnrollmentEvent[]
}

export interface PlannedStep {
  enrollmentId: string
  contactId: string
  sequenceId: string
  stepId: string
  dueAt: string
  state: 'sent' | 'skipped' | 'pending'
  subject: string | null
  template: string
}

export function normalizeSequence(input: Sequence): Required<Pick<Sequence, 'id' | 'steps'>> & Pick<Sequence, 'name'>
export function createEnrollment(sequence: Sequence, input: { id: string; contact: Contact; enrolledAt?: string | Date }, options?: { at?: string | Date | null }): Enrollment
export function normalizeEnrollment(input: Enrollment): Enrollment
export function recordEvent(enrollment: Enrollment, event: EnrollmentEvent): Enrollment
export function planEnrollment(sequence: Sequence, enrollment: Enrollment): PlannedStep[]
export function nextActions(sequence: Sequence, enrollments: Enrollment[], options?: { at?: string | Date | null }): PlannedStep[]
export function renderTemplate(template: string, variables?: Record<string, unknown>): { text: string; missing: string[] }
