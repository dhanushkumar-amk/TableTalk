/**
 * Typed UI events emitted by cards.
 *
 * Cards do NOT invoke MCP tools or backend endpoints directly.
 * Instead, they emit strongly typed UI events to a host listener (e.g., Alexa+ simulator in Phase 17).
 */

export interface AcceptOptionEvent {
  optionId: string;
  holdId?: string;
}

export interface HoldOptionEvent {
  optionId: string;
}

export interface SelectAlternativeEvent {
  date: string;
  time: string;
  area: string;
}

export interface CancelBookingEvent {
  confirmationCode: string;
  guestName: string;
}

export interface CancelOrderEvent {
  orderCode: string;
  guestName: string;
}

export interface OpenCheckoutEvent {
  checkoutUrl: string;
}

export interface CheckStatusEvent {
  type: "waitlist" | "deposit" | "booking";
  id: string;
}

export interface LeaveWaitlistEvent {
  waitlistId: string;
}

export interface CopyCodeEvent {
  code: string;
  label: string;
}

export interface CardEventHandlers {
  onAcceptOption?: (event: AcceptOptionEvent) => void;
  onHoldOption?: (event: HoldOptionEvent) => void;
  onSelectAlternative?: (event: SelectAlternativeEvent) => void;
  onCancelBooking?: (event: CancelBookingEvent) => void;
  onCancelOrder?: (event: CancelOrderEvent) => void;
  onOpenCheckout?: (event: OpenCheckoutEvent) => void;
  onCheckStatus?: (event: CheckStatusEvent) => void;
  onLeaveWaitlist?: (event: LeaveWaitlistEvent) => void;
  onCopyCode?: (event: CopyCodeEvent) => void;
}
