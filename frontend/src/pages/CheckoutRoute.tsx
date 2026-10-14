import { useParams } from "react-router-dom"
import CheckoutPage from "./CheckoutPage"

export function CheckoutRoute() {
  const { token } = useParams<{ token: string }>()
  return <CheckoutPage token={token ?? ""} />
}
