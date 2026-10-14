/**
 * Generates an iCalendar (.ics) download for a confirmed dining reservation.
 * Created entirely locally without remote CDNs or external services.
 */

export interface CalendarEventProps {
  title: string
  description: string
  location: string
  startDateIso: string // e.g. "2026-10-16T19:30:00"
  durationMinutes: number
  confirmationCode: string
}

export function downloadCalendarEvent({
  title,
  description,
  location,
  startDateIso,
  durationMinutes,
  confirmationCode,
}: CalendarEventProps): void {
  const start = new Date(startDateIso)
  const end = new Date(start.getTime() + durationMinutes * 60 * 1000)

  const formatIcsDate = (date: Date): string => {
    return date.toISOString().replace(/[-:]/g, "").split(".")[0] + "Z"
  }

  const icsContent = [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "PRODID:-//TableTalk//Dining Reservations//EN",
    "CALSCALE:GREGORIAN",
    "METHOD:PUBLISH",
    "BEGIN:VEVENT",
    `UID:tabletalk-${confirmationCode}-${start.getTime()}@tabletalk.local`,
    `DTSTAMP:${formatIcsDate(new Date())}`,
    `DTSTART:${formatIcsDate(start)}`,
    `DTEND:${formatIcsDate(end)}`,
    `SUMMARY:${title}`,
    `DESCRIPTION:${description.replace(/\n/g, "\\n")} - Confirmation Code: ${confirmationCode}`,
    `LOCATION:${location}`,
    "STATUS:CONFIRMED",
    "END:VEVENT",
    "END:VCALENDAR",
  ].join("\r\n")

  const blob = new Blob([icsContent], { type: "text/calendar;charset=utf-8" })
  const url = URL.createObjectURL(blob)
  const link = document.createElement("a")
  link.href = url
  link.setAttribute("download", `reservation-${confirmationCode}.ics`)
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}
