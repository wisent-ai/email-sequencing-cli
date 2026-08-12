const TERMINAL_EVENT_TYPES = new Set(['replied', 'bounced', 'unsubscribed'])
const STEP_EVENT_TYPES = new Set(['sent', 'skipped'])

function requiredString(value, label) {
  const normalized = String(value ?? '').trim()
  if (!normalized) throw new TypeError(`${label} must be a non-empty string`)
  return normalized
}

function optionalString(value) {
  const normalized = String(value ?? '').trim()
  return normalized || null
}

function isoDate(value, label) {
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) throw new TypeError(`${label} must be a valid date`)
  return date.toISOString()
}

function nonNegativeNumber(value, label) {
  const number = Number(value)
  if (!Number.isFinite(number) || number < 0) throw new TypeError(`${label} must be a non-negative number`)
  return number
}

function eventTime(event) {
  return new Date(event.at).getTime()
}

export function normalizeSequence(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new TypeError('sequence must be an object')
  if (!Array.isArray(input.steps) || input.steps.length === 0) throw new TypeError('sequence.steps must be a non-empty array')

  const seen = new Set()
  const steps = input.steps.map((step, index) => {
    if (!step || typeof step !== 'object' || Array.isArray(step)) throw new TypeError(`sequence.steps[${index}] must be an object`)
    const id = requiredString(step.id, `sequence.steps[${index}].id`)
    if (seen.has(id)) throw new TypeError(`duplicate sequence step id: ${id}`)
    seen.add(id)
    return {
      id,
      afterHours: nonNegativeNumber(step.afterHours ?? 0, `sequence.steps[${index}].afterHours`),
      subject: optionalString(step.subject),
      template: requiredString(step.template, `sequence.steps[${index}].template`),
    }
  })

  return {
    id: requiredString(input.id, 'sequence.id'),
    name: optionalString(input.name),
    steps,
  }
}

export function createEnrollment(sequenceInput, input, options = {}) {
  const sequence = normalizeSequence(sequenceInput)
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new TypeError('enrollment must be an object')
  const contact = input.contact
  if (!contact || typeof contact !== 'object' || Array.isArray(contact)) throw new TypeError('enrollment.contact must be an object')

  return {
    id: requiredString(input.id, 'enrollment.id'),
    sequenceId: sequence.id,
    contact: {
      id: requiredString(contact.id, 'enrollment.contact.id'),
      email: optionalString(contact.email),
      variables: contact.variables && typeof contact.variables === 'object' && !Array.isArray(contact.variables)
        ? { ...contact.variables }
        : {},
    },
    enrolledAt: isoDate(options.at ?? input.enrolledAt ?? new Date(), 'enrollment.enrolledAt'),
    status: 'active',
    events: [],
  }
}

export function normalizeEnrollment(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new TypeError('enrollment must be an object')
  const contact = input.contact
  if (!contact || typeof contact !== 'object' || Array.isArray(contact)) throw new TypeError('enrollment.contact must be an object')
  const status = requiredString(input.status ?? 'active', 'enrollment.status')
  if (!['active', 'paused', 'completed', 'cancelled'].includes(status)) throw new TypeError(`unsupported enrollment status: ${status}`)

  const events = Array.isArray(input.events) ? input.events.map((event, index) => {
    if (!event || typeof event !== 'object' || Array.isArray(event)) throw new TypeError(`enrollment.events[${index}] must be an object`)
    const type = requiredString(event.type, `enrollment.events[${index}].type`)
    const stepId = optionalString(event.stepId)
    if (STEP_EVENT_TYPES.has(type) && !stepId) throw new TypeError(`${type} events require stepId`)
    return {
      type,
      at: isoDate(event.at, `enrollment.events[${index}].at`),
      stepId,
      messageId: optionalString(event.messageId),
      reason: optionalString(event.reason),
    }
  }) : []

  events.sort((left, right) => eventTime(left) - eventTime(right))
  return {
    id: requiredString(input.id, 'enrollment.id'),
    sequenceId: requiredString(input.sequenceId, 'enrollment.sequenceId'),
    contact: {
      id: requiredString(contact.id, 'enrollment.contact.id'),
      email: optionalString(contact.email),
      variables: contact.variables && typeof contact.variables === 'object' && !Array.isArray(contact.variables)
        ? { ...contact.variables }
        : {},
    },
    enrolledAt: isoDate(input.enrolledAt, 'enrollment.enrolledAt'),
    status,
    events,
  }
}

export function recordEvent(enrollmentInput, event) {
  const enrollment = normalizeEnrollment(enrollmentInput)
  const next = normalizeEnrollment({ ...enrollment, events: [...enrollment.events, event] })
  if (next.events.some((item) => TERMINAL_EVENT_TYPES.has(item.type))) next.status = 'completed'
  return next
}

export function planEnrollment(sequenceInput, enrollmentInput) {
  const sequence = normalizeSequence(sequenceInput)
  const enrollment = normalizeEnrollment(enrollmentInput)
  if (sequence.id !== enrollment.sequenceId) throw new TypeError('sequence and enrollment sequenceId do not match')

  let dueAt = new Date(enrollment.enrolledAt).getTime()
  const sent = new Set(enrollment.events.filter((event) => event.type === 'sent').map((event) => event.stepId))
  const skipped = new Set(enrollment.events.filter((event) => event.type === 'skipped').map((event) => event.stepId))
  return sequence.steps.map((step) => {
    dueAt += step.afterHours * 60 * 60 * 1000
    return {
      enrollmentId: enrollment.id,
      contactId: enrollment.contact.id,
      sequenceId: sequence.id,
      stepId: step.id,
      dueAt: new Date(dueAt).toISOString(),
      state: sent.has(step.id) ? 'sent' : skipped.has(step.id) ? 'skipped' : 'pending',
      subject: step.subject,
      template: step.template,
    }
  })
}

export function nextActions(sequenceInput, enrollmentInputs, options = {}) {
  if (!Array.isArray(enrollmentInputs)) throw new TypeError('enrollments must be an array')
  const at = new Date(isoDate(options.at ?? new Date(), 'options.at')).getTime()
  const actions = []

  for (const input of enrollmentInputs) {
    const enrollment = normalizeEnrollment(input)
    if (enrollment.status !== 'active') continue
    if (enrollment.events.some((event) => TERMINAL_EVENT_TYPES.has(event.type))) continue
    const pending = planEnrollment(sequenceInput, enrollment).find((step) => step.state === 'pending')
    if (pending && new Date(pending.dueAt).getTime() <= at) actions.push(pending)
  }

  return actions.sort((left, right) => left.dueAt.localeCompare(right.dueAt) || left.enrollmentId.localeCompare(right.enrollmentId))
}

export function renderTemplate(template, variables = {}) {
  const missing = new Set()
  const text = requiredString(template, 'template').replace(/\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/gu, (_match, key) => {
    const value = key.split('.').reduce((current, part) => current && typeof current === 'object' ? current[part] : undefined, variables)
    if (value === undefined || value === null) {
      missing.add(key)
      return ''
    }
    return String(value)
  })
  return { text, missing: [...missing].sort() }
}
