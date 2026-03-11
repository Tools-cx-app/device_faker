// eslint-disable-next-line @typescript-eslint/no-explicit-any
if (!(Object as any).hasOwn) {
  Object.defineProperty(Object, 'hasOwn', {
    value(object: unknown, property: PropertyKey) {
      if (object == null) {
        throw new TypeError('Cannot convert undefined or null to object')
      }
      return Object.prototype.hasOwnProperty.call(Object(object), property)
    },
    configurable: true,
    enumerable: false,
    writable: true,
  })
}
