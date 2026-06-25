import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { ErrorBanner } from '../src/errorBanner.js'

describe('ErrorBanner', () => {
  let element
  let banner

  beforeEach(() => {
    vi.useFakeTimers()
    document.body.innerHTML = '<div id="banner"></div>'
    element = document.getElementById('banner')
    banner = new ErrorBanner(element)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('adds visible class when show() is called', () => {
    banner.show('oops')
    expect(element.classList.contains('visible')).toBe(true)
    expect(element.textContent).toBe('oops')
  })

  it('hides after the duration elapses', () => {
    banner.show('oops', 1000)
    expect(element.classList.contains('visible')).toBe(true)
    vi.advanceTimersByTime(1000)
    expect(element.classList.contains('visible')).toBe(false)
  })

  it('resets the timer when show() is called twice in a row', () => {
    banner.show('first', 1000)
    vi.advanceTimersByTime(800)
    banner.show('second', 1000)
    vi.advanceTimersByTime(800)
    expect(element.classList.contains('visible')).toBe(true)
    vi.advanceTimersByTime(200)
    expect(element.classList.contains('visible')).toBe(false)
  })
})
