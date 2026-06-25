const AUTO_HIDE_MS = 3000

export class HUD {
  constructor(element) {
    this.element = element
    this.timer = null
  }

  show() {
    this.element.classList.add('visible')
    if (this.timer) clearTimeout(this.timer)
    this.timer = setTimeout(() => this.hide(), AUTO_HIDE_MS)
  }

  hide() {
    this.element.classList.remove('visible')
    if (this.timer) {
      clearTimeout(this.timer)
      this.timer = null
    }
  }
}