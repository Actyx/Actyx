/*	$NetBSD: if_ether.h,v 1.34 2007/12/25 18:33:46 perry Exp $	*/

/*
 * Copyright (c) 1982, 1986, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)if_ether.h	8.3 (Berkeley) 5/2/95
 */

#ifndef _NETINET_IF_ETHER_H_
#define _NETINET_IF_ETHER_H_

#include <sys/cdefs.h>
#include <sys/types.h>

#if defined(__USE_BSD)

/* pull in Ethernet-specific definitions and packet structures */

#include <linux/if_ether.h>

/* pull in ARP-specific definitions and packet structures */

#include <net/if_arp.h>

#include <net/ethernet.h>

/* ... and define some more which we don't need anymore: */

/*
 * Ethernet Address Resolution Protocol.
 *
 * See RFC 826 for protocol description.  Structure below is not
 * used by our kernel!!! Only for userland programs which are externally
 * maintained and need it.
 */

struct	ether_arp {
	struct	 arphdr ea_hdr;			/* fixed-size header */
	u_int8_t arp_sha[ETHER_ADDR_LEN];	/* sender hardware address */
	u_int8_t arp_spa[4];			/* sender protocol address */
	u_int8_t arp_tha[ETHER_ADDR_LEN];	/* target hardware address */
	u_int8_t arp_tpa[4];			/* target protocol address */
} __packed;
#define	arp_hrd	ea_hdr.ar_hrd
#define	arp_pro	ea_hdr.ar_pro
#define	arp_hln	ea_hdr.ar_hln
#define	arp_pln	ea_hdr.ar_pln
#define	arp_op	ea_hdr.ar_op

/*
 * Macro to map an IP multicast address to an Ethernet multicast address.
 * The high-order 25 bits of the Ethernet address are statically assigned,
 * and the low-order 23 bits are taken from the low end of the IP address.
 */
#define ETHER_MAP_IP_MULTICAST(ipaddr, enaddr)				\
	/* struct in_addr *ipaddr; */					\
	/* u_int8_t enaddr[ETHER_ADDR_LEN]; */				\
{									\
	(enaddr)[0] = 0x01;						\
	(enaddr)[1] = 0x00;						\
	(enaddr)[2] = 0x5e;						\
	(enaddr)[3] = ((u_int8_t *)ipaddr)[1] & 0x7f;			\
	(enaddr)[4] = ((u_int8_t *)ipaddr)[2];				\
	(enaddr)[5] = ((u_int8_t *)ipaddr)[3];				\
}
/*
 * Macro to map an IP6 multicast address to an Ethernet multicast address.
 * The high-order 16 bits of the Ethernet address are statically assigned,
 * and the low-order 32 bits are taken from the low end of the IP6 address.
 */
#define ETHER_MAP_IPV6_MULTICAST(ip6addr, enaddr)			\
	/* struct in6_addr *ip6addr; */					\
	/* u_int8_t enaddr[ETHER_ADDR_LEN]; */				\
{                                                                       \
	(enaddr)[0] = 0x33;						\
	(enaddr)[1] = 0x33;						\
	(enaddr)[2] = ((u_int8_t *)ip6addr)[12];			\
	(enaddr)[3] = ((u_int8_t *)ip6addr)[13];			\
	(enaddr)[4] = ((u_int8_t *)ip6addr)[14];			\
	(enaddr)[5] = ((u_int8_t *)ip6addr)[15];			\
}

#endif /* __USE_BSD */

#endif /* !_NET_IF_ETHER_H_ */
