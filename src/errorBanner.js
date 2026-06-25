export class ErrorBanner {
  constructor(element) {
    this.element = element
    this.timer = null
  }

  show(message, duration = 3000) {
    this.element.textContent = message
    this.element.classList.add('visible')

    if (this.timer) clearTimeout(this.timer)
    this.timer = setTimeout(() => this.hide(), duration)
  }

  hide() {
    this.element.classList.remove('visible')
    if (this.timer) {
      clearTimeout(this.timer)
      this.timer = null
    }
  }
}
