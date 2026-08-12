#!/usr/bin/env node

import { readFile } from 'node:fs/promises'
import { createEnrollment, nextActions, planEnrollment, recordEvent, renderTemplate } from './index.js'

function usage() {
  return `email-sequencing-cli

Usage:
  email-sequencing enroll --sequence <sequence.json> --enrollment <enrollment.json> [--at <ISO-8601>]
  email-sequencing plan --sequence <sequence.json> --enrollments <enrollments.json> [--at <ISO-8601>]
  email-sequencing schedule --sequence <sequence.json> --enrollment <enrollment.json>
  email-sequencing record --enrollment <enrollment.json> --event <event.json>
  email-sequencing render --template <template.txt> --variables <variables.json>

Commands print JSON to stdout and never send email.`
}

function value(args, name, required = true) {
  const index = args.indexOf(name)
  const result = index >= 0 ? args[index + 1] : null
  if (required && !result) throw new Error(`${name} is required`)
  return result
}

async function json(path) {
  return JSON.parse(await readFile(path, 'utf8'))
}

async function main() {
  const args = process.argv.slice(2)
  if (!args.length || args.includes('--help') || args.includes('-h')) {
    console.log(usage())
    return
  }

  const command = args[0]
  let output
  if (command === 'enroll') {
    output = createEnrollment(
      await json(value(args, '--sequence')),
      await json(value(args, '--enrollment')),
      { at: value(args, '--at', false) },
    )
  } else if (command === 'plan') {
    output = nextActions(
      await json(value(args, '--sequence')),
      await json(value(args, '--enrollments')),
      { at: value(args, '--at', false) },
    )
  } else if (command === 'schedule') {
    output = planEnrollment(
      await json(value(args, '--sequence')),
      await json(value(args, '--enrollment')),
    )
  } else if (command === 'record') {
    output = recordEvent(
      await json(value(args, '--enrollment')),
      await json(value(args, '--event')),
    )
  } else if (command === 'render') {
    output = renderTemplate(
      await readFile(value(args, '--template'), 'utf8'),
      await json(value(args, '--variables')),
    )
  } else {
    throw new Error(`Unknown command: ${command}\n\n${usage()}`)
  }
  console.log(JSON.stringify(output, null, 2))
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
